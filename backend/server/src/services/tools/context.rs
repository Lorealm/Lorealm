use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use lorealm_storage::models::AGENTS_SCOPE;
use weaver_lang::{EvalContext, EvalError, Registry, Value as WeaverValue};

use crate::{
    services::state::write_var,
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

pub struct ToolContext {
    cmd: HashMap<String, WeaverValue>,
    local: HashMap<String, WeaverValue>,
    state: Arc<Mutex<ToolStatePool>>,
    read_only: bool,
    actor: Option<Actor>,
}

impl ToolContext {
    pub fn new(cmd: HashMap<String, WeaverValue>, state: Arc<Mutex<ToolStatePool>>) -> Self {
        Self {
            cmd,
            local: HashMap::new(),
            state,
            read_only: false,
            actor: None,
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

    /// The pool variable a `self:`-aliased name maps to:
    /// `agents:<handle>.<name>`.
    fn own_var(&self, name: &str) -> Option<String> {
        self.actor
            .as_ref()
            .filter(|a| !a.handle.is_empty())
            .map(|a| format!("{}.{name}", a.handle))
    }

    fn state_var(&self, scope: &str, name: &str) -> Result<Option<WeaverValue>, EvalError> {
        let pool = self.state.lock().unwrap();
        let Some(ns) = pool.get(scope) else {
            return Ok(None);
        };
        let found = match name.split_once('.') {
            None => ns.get(name),
            Some((root, rest)) => {
                let mut cur = ns.get(root);
                for segment in rest.split('.') {
                    cur = cur.and_then(|v| v.get(segment));
                }
                cur
            }
        };
        found
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
            "cmd" => Ok(self.cmd.get(name).cloned()),
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
}
