use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use lorealm_storage::models::AGENTS_SCOPE;
use weaver_lang::{
    EvalContext, EvalError, Registry, Value as WeaverValue, WeaverCommand,
    registry::{CommandSignature, ParamDef, ValueType},
};

use crate::{
    services::state::{delete_var, write_var},
    util::weaver::{json_to_weaver, weaver_to_json},
};

pub type ToolStatePool =
    std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>;

/// The realm agent a weaver body runs on behalf of.
#[derive(Clone)]
pub struct Actor {
    pub handle: String,
    pub name: String,
    /// [`AgentKind::as_str`]: `llm` | `player` | `script`.
    ///
    /// [`AgentKind::as_str`]: lorealm_storage::models::AgentKind::as_str
    pub kind: &'static str,
}

/// The realm a weaver body runs inside, as the `cast:` and `turn:` namespaces
/// see it.
#[derive(Clone, Default)]
pub struct RealmView(Arc<ToolStatePool>);

/// The namespaces [`RealmView`] owns. Reserved in every [`ToolContext`], with
/// or without a view behind them, the way `actor:` is.
pub const REALM_SCOPES: [&str; 3] = ["cast", "turn", "scenario"];

impl RealmView {
    pub fn new(pool: ToolStatePool) -> Self {
        Self(Arc::new(pool))
    }

    pub(crate) fn get(&self, scope: &str, name: &str) -> Option<&serde_json::Value> {
        lookup(self.0.get(scope)?, name)
    }
}

/// Follow a dotted `name` into one namespace of a pool.
fn lookup<'a>(
    ns: &'a std::collections::HashMap<String, serde_json::Value>,
    name: &str,
) -> Option<&'a serde_json::Value> {
    match name.split_once('.') {
        None => ns.get(name),
        Some((root, rest)) => {
            let mut cur = ns.get(root);
            for segment in rest.split('.') {
                cur = cur.and_then(|v| v.get(segment));
            }
            cur
        }
    }
}

pub struct ToolContext {
    cmd: HashMap<String, WeaverValue>,
    local: HashMap<String, WeaverValue>,
    state: Arc<Mutex<ToolStatePool>>,
    read_only: bool,
    actor: Option<Actor>,
    realm: Option<RealmView>,
}

impl ToolContext {
    pub fn new(cmd: HashMap<String, WeaverValue>, state: Arc<Mutex<ToolStatePool>>) -> Self {
        Self {
            cmd,
            local: HashMap::new(),
            state,
            read_only: false,
            actor: None,
            realm: None,
        }
    }

    /// Reject every state write (`local:` scratch stays writable). Used for
    /// run-condition evaluation.
    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }

    pub fn with_actor(mut self, actor: Option<Actor>) -> Self {
        self.actor = actor;
        self
    }

    /// Mount `cast:` and `turn:` for a body running inside a realm.
    pub fn with_realm(mut self, realm: Option<RealmView>) -> Self {
        self.realm = realm;
        self
    }

    /// The pool variable a `self:`-aliased name maps to:
    /// `agents:<handle>.<name>`.
    fn own_var(&self, name: &str) -> Option<String> {
        self.actor
            .as_ref()
            .filter(|a| !a.handle.is_empty())
            .map(|a| format!("{}.{name}", a.handle))
    }

    /// Remove `scope:name` from the pool, following a dotted path into an
    /// object if one is present.
    pub fn delete_variable(&mut self, scope: &str, name: &str) -> Result<(), EvalError> {
        let (scope, name) = match scope {
            "self" => match self.own_var(name) {
                Some(var) => (AGENTS_SCOPE, var),
                None => {
                    return Err(EvalError::host_error(
                        "'self:' has no bound agent identity here",
                    ));
                }
            },
            other => (other, name.to_string()),
        };
        match scope {
            "local" => {
                self.local.remove(&name);
                Ok(())
            }
            "user" | "char" | "actor" => Err(EvalError::host_error(format!(
                "namespace '{scope}' is a reserved namespace"
            ))),
            _ if REALM_SCOPES.contains(&scope) => Err(EvalError::host_error(format!(
                "namespace '{scope}' is the realm's own, and is read-only"
            ))),
            other => {
                if self.read_only {
                    return Err(EvalError::host_error(format!(
                        "'{other}:{name}' cannot be deleted in a read-only evaluation"
                    )));
                }
                let mut pool = self.state.lock().unwrap();
                if let Some(ns) = pool.get_mut(other) {
                    delete_var(ns, &name);
                }
                Ok(())
            }
        }
    }

    fn state_var(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        let pool = self.state.lock().unwrap();
        let Some(ns) = pool.get(scope) else {
            return Ok(None);
        };
        lookup(ns, name)
            .map(|v| json_to_weaver(v).map_err(|e| EvalError::host_error(e.to_string())))
            .transpose()
    }

    /// A `cast:`/`turn:` read. Absent without a mounted view, the same way
    /// `actor:` is absent without an actor.
    fn realm_var(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        self.realm
            .as_ref()
            .and_then(|r| r.get(scope, name))
            .map(|v| json_to_weaver(v).map_err(|e| EvalError::host_error(e.to_string())))
            .transpose()
    }

    fn own_state_var(&self, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        match self.own_var(name) {
            Some(var) => self.state_var(AGENTS_SCOPE, &var),
            None => Ok(None),
        }
    }
}

impl EvalContext for ToolContext {
    fn resolve_variable(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        match scope {
            // `props:` is the widget-render spelling of the same bound map.
            "cmd" | "props" => Ok(self.cmd.get(name).cloned()),
            "local" => Ok(self.local.get(name).cloned()),
            "self" => self.own_state_var(name),
            "actor" => match &self.actor {
                Some(a) => match name {
                    "handle" => Ok(Some(WeaverValue::String(a.handle.clone()))),
                    "name" => Ok(Some(WeaverValue::String(a.name.clone()))),
                    "kind" => Ok(Some(WeaverValue::String(a.kind.to_string()))),
                    prop => self.own_state_var(prop),
                },
                None => Ok(None),
            },
            _ if REALM_SCOPES.contains(&scope) => self.realm_var(scope, name),
            other => self.state_var(other, name),
        }
    }

    fn set_variable(
        &mut self,
        scope: &str,
        name: &str,
        value: WeaverValue,
    ) -> Result<(), EvalError> {
        // `self:` writes map back onto the agent's own subtree.
        let (scope, name) = match scope {
            "self" => match self.own_var(name) {
                Some(var) => (AGENTS_SCOPE, var),
                None => {
                    return Err(EvalError::host_error(
                        "'self:' has no bound agent identity here",
                    ));
                }
            },
            other => (other, name.to_string()),
        };
        match scope {
            "local" => {
                self.local.insert(name, value);
                Ok(())
            }
            "user" | "char" | "actor" => Err(EvalError::host_error(format!(
                "namespace '{scope}' is a reserved namespace"
            ))),
            _ if REALM_SCOPES.contains(&scope) => Err(EvalError::host_error(format!(
                "namespace '{scope}' is the realm's own, and is read-only"
            ))),
            other => {
                if self.read_only {
                    return Err(EvalError::host_error(format!(
                        "'{other}:{name}' cannot be written in a read-only evaluation"
                    )));
                }
                let json =
                    weaver_to_json(&value).map_err(|e| EvalError::host_error(e.to_string()))?;
                let mut pool = self.state.lock().unwrap();
                write_var(pool.entry(other.into()).or_default(), &name, &json);
                Ok(())
            }
        }
    }

    fn fire_trigger(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        // TODO: Eventually we want to tie this into lorebooks
        Err(EvalError::host_error(
            "triggers are currently not supported in tool bodies",
        ))
    }
    fn resolve_document(&mut self, _: &str, _: &Registry) -> Result<String, EvalError> {
        // TODO: Eventually we want to tie this into lorebooks
        Err(EvalError::host_error(
            "documents are currently not supported in tool bodies",
        ))
    }
}

/// `$[del_var("scope:name")]`: remove a variable, or one key inside it when
/// the name carries a dotted path.
pub struct DelVarCommand;

impl WeaverCommand for DelVarCommand {
    fn call(
        &self,
        args: Vec<WeaverValue>,
        ctx: &mut dyn EvalContext,
        _registry: &Registry,
    ) -> Result<Option<WeaverValue>, EvalError> {
        let key = match args.into_iter().next() {
            Some(WeaverValue::String(s)) => s,
            other => {
                return Err(EvalError::host_error(format!(
                    "del_var: key must be a \"scope:name\" string, got {other:?}"
                )));
            }
        };
        let (scope, name) = key.split_once(':').ok_or_else(|| {
            EvalError::host_error(format!(
                "del_var: invalid variable key \"{key}\": expected \"scope:name\" format"
            ))
        })?;

        let any: &mut dyn std::any::Any = ctx;
        any.downcast_mut::<ToolContext>()
            .ok_or_else(|| EvalError::host_error("del_var: not available in this context"))?
            .delete_variable(scope, name)?;
        Ok(None)
    }

    fn signature(&self) -> CommandSignature {
        CommandSignature {
            name: "del_var".into(),
            description: "Remove a variable, or one key inside it when the name is a dotted path"
                .into(),
            returns: ValueType::None,
            params: vec![ParamDef {
                name: "key".into(),
                description: "The variable to remove, as \"scope:name\"".into(),
                expected_type: Some(ValueType::String),
                required: true,
            }],
            mutates_context: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ToolContext {
        let state = Arc::new(Mutex::new(ToolStatePool::default()));
        ToolContext::new(HashMap::new(), state)
    }

    #[test]
    fn writes_land_anywhere_but_reserved_namespaces() {
        let mut c = ctx();
        assert!(
            c.set_variable("nation", "a.gold", WeaverValue::None)
                .is_ok()
        );
        assert!(c.set_variable("user", "name", WeaverValue::None).is_err());
        assert!(c.set_variable("char", "name", WeaverValue::None).is_err());
    }

    #[test]
    fn read_only_blocks_state_writes_but_not_local() {
        let mut c = ctx().read_only();
        assert!(
            c.set_variable("nation", "a.gold", WeaverValue::None)
                .is_err()
        );
        assert!(
            c.set_variable("local", "scratch", WeaverValue::None)
                .is_ok()
        );
    }

    fn actor_ctx() -> ToolContext {
        ctx().with_actor(Some(Actor {
            handle: "h".into(),
            name: "Tester".into(),
            kind: "llm",
        }))
    }

    #[test]
    fn self_maps_onto_the_own_agents_subtree() {
        let mut c = actor_ctx();
        c.set_variable("self", "gold", WeaverValue::Number(5.0))
            .unwrap();
        // The write landed at agents:h.gold and reads back through both paths.
        assert_eq!(
            c.resolve_variable("agents", "h.gold").unwrap(),
            Some(WeaverValue::Number(5.0))
        );
        assert_eq!(
            c.resolve_variable("self", "gold").unwrap(),
            Some(WeaverValue::Number(5.0))
        );
    }

    fn realm_ctx() -> ToolContext {
        ctx().with_realm(Some(RealmView::new(ToolStatePool::from([
            (
                "cast".to_string(),
                std::collections::HashMap::from([
                    ("count".to_string(), serde_json::json!(2)),
                    (
                        "llm".to_string(),
                        serde_json::json!({ "count": 1, "handles": ["alice"] }),
                    ),
                ]),
            ),
            (
                "turn".to_string(),
                std::collections::HashMap::from([("index".to_string(), serde_json::json!(3))]),
            ),
        ]))))
    }

    #[test]
    fn the_realm_view_reads_through_dotted_paths() {
        let c = realm_ctx();
        assert_eq!(
            c.resolve_variable("cast", "count").unwrap(),
            Some(WeaverValue::Number(2.0))
        );
        assert_eq!(
            c.resolve_variable("cast", "llm.count").unwrap(),
            Some(WeaverValue::Number(1.0))
        );
        assert_eq!(
            c.resolve_variable("turn", "index").unwrap(),
            Some(WeaverValue::Number(3.0))
        );
        assert_eq!(c.resolve_variable("cast", "llm.missing").unwrap(), None);
    }

    #[test]
    fn the_realm_scopes_are_reserved_with_or_without_a_view() {
        assert_eq!(ctx().resolve_variable("cast", "count").unwrap(), None);

        for scope in REALM_SCOPES {
            assert!(
                realm_ctx()
                    .set_variable(scope, "count", WeaverValue::Number(9.0))
                    .is_err(),
                "{scope} accepted a write"
            );
        }
    }

    #[test]
    fn actor_mounts_identity_and_properties_read_only() {
        let mut c = actor_ctx();
        c.set_variable("self", "gold", WeaverValue::Number(5.0))
            .unwrap();
        assert_eq!(
            c.resolve_variable("actor", "handle").unwrap(),
            Some(WeaverValue::String("h".into()))
        );
        assert_eq!(
            c.resolve_variable("actor", "kind").unwrap(),
            Some(WeaverValue::String("llm".into()))
        );
        assert_eq!(
            c.resolve_variable("actor", "gold").unwrap(),
            Some(WeaverValue::Number(5.0))
        );
        assert!(
            c.set_variable("actor", "handle", WeaverValue::None)
                .is_err()
        );
    }

    #[test]
    fn self_without_an_actor_is_unresolvable_and_unwritable() {
        let mut c = ctx();
        assert_eq!(c.resolve_variable("self", "gold").unwrap(), None);
        assert!(c.set_variable("self", "gold", WeaverValue::None).is_err());
    }

    /// A context holding `nation:oda = { gold: 10, men: 5 }`.
    fn nation_ctx() -> ToolContext {
        let mut c = ctx();
        c.set_variable("nation", "oda.gold", WeaverValue::Number(10.0))
            .unwrap();
        c.set_variable("nation", "oda.men", WeaverValue::Number(5.0))
            .unwrap();
        c
    }

    #[test]
    fn deleting_a_dotted_path_leaves_its_siblings() {
        let mut c = nation_ctx();
        c.delete_variable("nation", "oda.gold").unwrap();

        assert_eq!(c.resolve_variable("nation", "oda.gold").unwrap(), None);
        assert_eq!(
            c.resolve_variable("nation", "oda.men").unwrap(),
            Some(WeaverValue::Number(5.0))
        );
    }

    #[test]
    fn deleting_a_bare_name_takes_the_whole_variable() {
        let mut c = nation_ctx();
        c.delete_variable("nation", "oda").unwrap();
        assert_eq!(c.resolve_variable("nation", "oda").unwrap(), None);
    }

    #[test]
    fn deleting_something_absent_is_a_no_op() {
        let mut c = nation_ctx();
        c.delete_variable("nation", "takeda").unwrap();
        c.delete_variable("nation", "oda.rice").unwrap();
        c.delete_variable("void", "anything").unwrap();
        assert_eq!(
            c.resolve_variable("nation", "oda.gold").unwrap(),
            Some(WeaverValue::Number(10.0))
        );
    }

    #[test]
    fn deletes_refuse_where_writes_do() {
        let mut c = nation_ctx();
        assert!(c.delete_variable("user", "name").is_err());
        assert!(c.delete_variable("char", "name").is_err());
        assert!(c.delete_variable("cast", "count").is_err());
        assert!(c.delete_variable("self", "gold").is_err()); // no actor bound
    }

    #[test]
    fn read_only_blocks_state_deletes_but_not_local() {
        let mut c = nation_ctx().read_only();
        assert!(c.delete_variable("nation", "oda").is_err());
        assert!(c.delete_variable("local", "scratch").is_ok());
    }

    #[test]
    fn self_deletes_land_on_the_own_agents_subtree() {
        let mut c = actor_ctx();
        c.set_variable("self", "gold", WeaverValue::Number(5.0))
            .unwrap();
        c.delete_variable("self", "gold").unwrap();
        assert_eq!(c.resolve_variable("agents", "h.gold").unwrap(), None);
    }

    #[test]
    fn del_var_runs_through_the_command() {
        let registry = Registry::new();
        let mut c = nation_ctx();

        DelVarCommand
            .call(
                vec![WeaverValue::String("nation:oda.gold".into())],
                &mut c,
                &registry,
            )
            .unwrap();
        assert_eq!(c.resolve_variable("nation", "oda.gold").unwrap(), None);

        // A key with no scope half, and a key that is not a string at all.
        assert!(
            DelVarCommand
                .call(vec![WeaverValue::String("oda".into())], &mut c, &registry)
                .is_err()
        );
        assert!(
            DelVarCommand
                .call(vec![WeaverValue::Number(1.0)], &mut c, &registry)
                .is_err()
        );
    }
}
