use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use lorealm_storage::models::{
    AGENTS_SCOPE, AgentGrant, Param, RealmAgent, ScopePattern, StateDelta,
};
use weaver_lang::{CompiledExpr, CompiledTemplate, Value as WeaverValue};

use crate::{
    errors::RealmError,
    services::{
        state::{StatePool, StateService},
        tools::{
            Actor, EmitEventCommand, EmittedEvent, EventSink, ToolContext, build_tool_registry,
        },
    },
    util::weaver::json_to_weaver,
};

/// The scopes an agent may read
pub(super) fn read_scopes(grants: &[AgentGrant]) -> Vec<ScopePattern> {
    grants
        .iter()
        .map(|g| ScopePattern::parse(&g.channel))
        .collect()
}

pub(super) fn read_scopes_of(agent: &RealmAgent) -> Vec<ScopePattern> {
    let mut scopes = read_scopes(&agent.grants);
    scopes.extend(agent.own_scope());
    scopes
}

pub(super) fn actor_of(agent: &RealmAgent) -> Actor {
    Actor {
        handle: agent.handle.clone(),
        name: agent.name.clone(),
        kind: agent.kind.as_str(),
    }
}

pub(super) fn inject_self_alias(agent: &RealmAgent, slice: &mut StatePool) {
    if agent.handle.is_empty() {
        return;
    }
    let prefix = format!("{}.", agent.handle);
    let own: HashMap<String, serde_json::Value> = slice
        .get(AGENTS_SCOPE)
        .map(|vars| {
            vars.iter()
                .filter_map(|(k, v)| k.strip_prefix(&prefix).map(|n| (n.to_string(), v.clone())))
                .collect()
        })
        .unwrap_or_default();
    if !own.is_empty() {
        slice.insert("self".into(), own);
    }
}

/// The subpool an agent may read
pub(super) fn slice(world: &StatePool, read: &[ScopePattern]) -> StatePool {
    world
        .iter()
        .filter_map(|(ns, vars)| {
            let kept: HashMap<_, _> = vars
                .iter()
                .filter(|(name, _)| read.iter().any(|g| g.matches_var(ns, name)))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            (!kept.is_empty()).then(|| (ns.clone(), kept))
        })
        .collect()
}

/// Compile and run weaver `source` against the WHOLE pool on behalf of
/// `agent`
pub(super) async fn run_scoped(
    state: &StateService,
    source: &str,
    params: &[Param],
    args: serde_json::Value,
    agent: &RealmAgent,
    world: &StatePool,
) -> Result<(String, StateDelta, Vec<EmittedEvent>), RealmError> {
    let before = world.clone();

    let body = CompiledTemplate::compile(source).map_err(|errors| {
        RealmError::ActionFailed(format!("body failed to compile: {errors:?}"))
    })?;

    // Bind declared params into the `cmd:` namespace the source reads. Scripts
    // declare none, so this is a no-op for them.
    let args = args.as_object().cloned().unwrap_or_default();
    let mut cmd = HashMap::new();
    for p in params {
        match args.get(&p.name) {
            Some(v) => {
                cmd.insert(p.name.clone(), json_to_weaver(v));
            }
            None if p.required => {
                return Err(RealmError::ActionFailed(format!(
                    "missing required parameter '{}'",
                    p.name
                )));
            }
            None => {
                cmd.insert(p.name.clone(), WeaverValue::None);
            }
        }
    }

    let pool = Arc::new(StdMutex::new(before.clone()));
    let mut ctx = ToolContext::new(cmd, Arc::clone(&pool)).with_actor(Some(actor_of(agent)));

    let sink: EventSink = Arc::new(StdMutex::new(Vec::new()));
    let mut registry = build_tool_registry();
    registry.register_command(EmitEventCommand::new(Arc::clone(&sink)));

    let output = body
        .evaluate(&mut ctx, &registry)
        .map_err(|e| RealmError::ActionFailed(e.to_string()))?;

    let delta = {
        let after = pool.lock().unwrap();
        state.diff(&before, &after)
    };
    let events = std::mem::take(&mut *sink.lock().unwrap());
    Ok((output, delta, events))
}

/// Compile and run weaver `source`
pub(super) fn run_seed(source: &str, pool: StatePool) -> Result<StatePool, RealmError> {
    let body = CompiledTemplate::compile(source).map_err(|errors| {
        RealmError::InvalidScenario(format!("seed script failed to compile: {errors:?}"))
    })?;
    let shared = Arc::new(StdMutex::new(pool));
    let mut ctx = ToolContext::new(HashMap::new(), Arc::clone(&shared));
    body.evaluate(&mut ctx, &build_tool_registry())
        .map_err(|e| RealmError::InvalidScenario(format!("seed script failed: {e}")))?;
    drop(ctx);
    Ok(Arc::try_unwrap(shared).unwrap().into_inner().unwrap())
}

/// Whether an agent should run on its turn
pub(super) fn run_condition_passes(agent: &RealmAgent, world: &StatePool) -> bool {
    let Some(src) = agent
        .run_condition
        .as_ref()
        .filter(|s| !s.trim().is_empty())
    else {
        return true;
    };
    let Ok(expr) = CompiledExpr::compile(src) else {
        return false;
    };

    let read = slice(world, &read_scopes_of(agent));
    let pool = Arc::new(StdMutex::new(read));
    let mut ctx = ToolContext::new(HashMap::new(), pool)
        .read_only()
        .with_actor(Some(actor_of(agent)));

    match expr.evaluate(&mut ctx, &build_tool_registry()) {
        Ok(v) => v.is_truthy(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lorealm_storage::models::AgentAccess;
    use serde_json::json;

    fn grant(scope: &str) -> AgentGrant {
        AgentGrant {
            channel: scope.into(),
            access: AgentAccess::Read,
        }
    }

    /// A script-kind agent with handle `h`, the given grants, and an optional
    /// run condition.
    fn agent(grants: Vec<AgentGrant>, run_condition: Option<&str>) -> RealmAgent {
        RealmAgent {
            id: lorealm_storage::models::Snowflake(1),
            name: "tester".into(),
            handle: "h".into(),
            kind: lorealm_storage::models::AgentKind::Script,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            user_id: None,
            persona_id: None,
            turn_order: 0,
            properties: HashMap::new(),
            grants,
            tools: Vec::new(),
            speech_channel: None,
            lorebooks: Vec::new(),
            widgets: Vec::new(),
            script: None,
            run_condition: run_condition.map(Into::into),
        }
    }

    fn condition_passes(condition: &str, grants: Vec<AgentGrant>, world: &StatePool) -> bool {
        run_condition_passes(&agent(grants, Some(condition)), world)
    }

    /// A world with `counter.n = 7`.
    fn world() -> StatePool {
        let mut w = StatePool::new();
        w.entry("counter".into())
            .or_default()
            .insert("n".into(), json!(7));
        w
    }

    #[test]
    fn slice_filters_per_variable() {
        let mut w = StatePool::new();
        let nation = w.entry("nation".into()).or_default();
        nation.insert("a.gold".into(), json!(100));
        nation.insert("b.gold".into(), json!(50));
        w.entry("market".into())
            .or_default()
            .insert("ore.price".into(), json!(3));

        let read = read_scopes(&[grant("nation:a.*")]);
        let s = slice(&w, &read);

        assert_eq!(s["nation"].len(), 1);
        assert!(s["nation"].contains_key("a.gold"));
        // `market` matched nothing: the namespace is dropped, not left empty.
        assert!(!s.contains_key("market"));
    }

    #[test]
    fn dotted_names_read_through_scoped_grants() {
        let mut w = StatePool::new();
        w.entry("nation".into())
            .or_default()
            .insert("a.gold".into(), json!(100));

        assert!(condition_passes(
            "{{nation:a.gold}} > 50",
            vec![grant("nation:a.*")],
            &w
        ));

        assert!(!condition_passes(
            "{{nation:a.gold}} > 50",
            vec![grant("nation:b.*")],
            &w
        ));
    }

    #[test]
    fn absent_or_empty_condition_always_runs() {
        assert!(run_condition_passes(
            &agent(vec![grant("counter")], None),
            &world()
        ));
        assert!(condition_passes("   ", vec![grant("counter")], &world()));
    }

    #[test]
    fn truthy_condition_runs_falsy_skips() {
        assert!(condition_passes(
            "{{counter:n}} > 5",
            vec![grant("counter")],
            &world()
        ));
        assert!(!condition_passes(
            "{{counter:n}} > 10",
            vec![grant("counter")],
            &world()
        ));
    }

    #[test]
    fn broken_expression_errors_closed() {
        // A malformed expression never runs the agent.
        assert!(!condition_passes(
            "{{counter:n} >",
            vec![grant("counter")],
            &world()
        ));
    }

    #[test]
    fn condition_reads_only_the_granted_slice() {
        // Without a grant on `counter`, its value is invisible
        assert!(!condition_passes(
            "{{counter:n}} > 5",
            vec![grant("other")],
            &world()
        ));
    }

    /// A world with `agents:h.stamina = 3` and `agents:rival.stamina = 9`.
    fn agents_world() -> StatePool {
        let mut w = StatePool::new();
        let agents = w.entry(AGENTS_SCOPE.into()).or_default();
        agents.insert("h.stamina".into(), json!(3));
        agents.insert("rival.stamina".into(), json!(9));
        w
    }

    #[test]
    fn own_subtree_is_readable_without_a_grant() {
        // `self:` resolves through the implicit `agents:h.*` grant; the
        // rival's subtree stays invisible without an explicit grant.
        assert!(condition_passes(
            "{{self:stamina}} == 3",
            vec![],
            &agents_world()
        ));
        assert!(condition_passes(
            "{{agents:h.stamina}} == 3",
            vec![],
            &agents_world()
        ));
        assert!(!condition_passes(
            "{{agents:rival.stamina}} == 9",
            vec![],
            &agents_world()
        ));
    }

    #[test]
    fn actor_identity_is_mounted() {
        assert!(condition_passes(
            r#"{{actor:handle}} == "h" && {{actor:kind}} == "script" && {{actor:stamina}} == 3"#,
            vec![],
            &agents_world()
        ));
    }

    #[test]
    fn run_seed_writes_into_the_pool_without_an_actor() {
        let pool = run_seed(r#"$[set_var("world:season", "winter")]"#, StatePool::new()).unwrap();
        assert_eq!(pool["world"]["season"], json!("winter"));
        // `self:` has no meaning in a seed script.
        assert!(run_seed(r#"$[set_var("self:x", 1)]"#, StatePool::new()).is_err());
    }

    #[test]
    fn self_alias_injection_strips_the_handle_prefix() {
        let a = agent(vec![], None);
        let mut s = slice(&agents_world(), &read_scopes_of(&a));
        inject_self_alias(&a, &mut s);
        assert_eq!(s["self"]["stamina"], json!(3));
        // Only the own subtree was granted and aliased.
        assert!(!s[AGENTS_SCOPE].contains_key("rival.stamina"));
    }
}
