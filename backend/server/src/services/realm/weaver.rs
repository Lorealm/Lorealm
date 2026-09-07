use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use lorealm_storage::models::{
    AGENTS_SCOPE, AgentGrant, Param, RealmAgent, ScopePattern, StateDelta,
};

use crate::{
    errors::RealmError,
    services::{
        state::StatePool,
        tools::{Actor, EmittedEvent},
        weaver::{
            WeaverError, WeaverHost, WeaverProfile, bind_json_args, compile_expr, compile_template,
        },
    },
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
    let own: HashMap<String, serde_json::Value> = slice
        .get(AGENTS_SCOPE)
        .and_then(|vars| vars.get(&agent.handle))
        .and_then(|own| own.as_object())
        .map(|fields| fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        .unwrap_or_default();
    if !own.is_empty() {
        slice.insert("self".into(), own);
    }
}

/// Keep the parts of `value` reachable at a granted dotted path.
///
/// `path` is the name the value sits at, rebuilt as the walk descends, so
/// grants are still asked the same question they were asked when the pool
/// was flat — [`ScopePattern::matches_var`] is unchanged.
///
/// A grant on a branch (`nation:a.*`) admits the whole subtree; a grant on a
/// leaf (`market:ore.price`) admits only that leaf. Returns `None` when
/// nothing under `value` is readable.
fn prune(
    ns: &str,
    path: &str,
    value: &serde_json::Value,
    read: &[ScopePattern],
) -> Option<serde_json::Value> {
    if read.iter().any(|g| g.matches_var(ns, path)) {
        return Some(value.clone());
    }
    // Not granted outright — a descendant still might be.
    let fields = value.as_object()?;
    let kept: serde_json::Map<_, _> = fields
        .iter()
        .filter_map(|(key, child)| {
            let child_path = format!("{path}.{key}");
            Some((key.clone(), prune(ns, &child_path, child, read)?))
        })
        .collect();
    (!kept.is_empty()).then(|| serde_json::Value::Object(kept))
}

/// The subpool an agent may read
pub(super) fn slice(world: &StatePool, read: &[ScopePattern]) -> StatePool {
    world
        .iter()
        .filter_map(|(ns, vars)| {
            let kept: HashMap<_, _> = vars
                .iter()
                .filter_map(|(name, v)| Some((name.clone(), prune(ns, name, v, read)?)))
                .collect();
            (!kept.is_empty()).then(|| (ns.clone(), kept))
        })
        .collect()
}

/// Compile and run weaver `source` against the WHOLE pool on behalf of
/// `agent`
pub(super) async fn run_scoped(
    source: &str,
    params: &[Param],
    args: serde_json::Value,
    agent: &RealmAgent,
    world: &StatePool,
) -> Result<(String, StateDelta, Vec<EmittedEvent>), RealmError> {
    // Declared params land in the `cmd:` namespace the source reads. Scripts
    // declare none, so this is a no-op for them.
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .args(bind_json_args(params, &args).map_err(action_failed)?)
        .actor(Some(actor_of(agent)))
        .pool(Arc::new(StdMutex::new(world.clone())))
        .build();

    let outcome = host
        .evaluate(compile_template(source).map_err(action_failed)?)
        .await
        .map_err(action_failed)?;

    Ok((outcome.to_output_string(), outcome.delta, outcome.events))
}

/// Compile and run a scenario's seed script, returning the pool it built.
pub(super) fn run_seed(source: &str, pool: StatePool) -> Result<StatePool, RealmError> {
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmSeed)
        .pool(Arc::new(StdMutex::new(pool)))
        .build();

    let outcome = host
        .evaluate_inline(compile_template(source).map_err(invalid_scenario)?)
        .map_err(invalid_scenario)?;

    Ok(outcome.pool)
}

fn action_failed(e: WeaverError) -> RealmError {
    RealmError::ActionFailed(e.to_string())
}

fn invalid_scenario(e: WeaverError) -> RealmError {
    RealmError::InvalidScenario(format!("seed script: {e}"))
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
    let Ok(expr) = compile_expr(src) else {
        return false;
    };

    let read = slice(world, &read_scopes_of(agent));
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmCondition)
        .actor(Some(actor_of(agent)))
        .pool(Arc::new(StdMutex::new(read)))
        .build();

    host.evaluate_inline(expr)
        .map(|outcome| outcome.is_truthy())
        .unwrap_or(false)
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
        nation.insert("a".into(), json!({ "gold": 100 }));
        nation.insert("b".into(), json!({ "gold": 50 }));
        w.entry("market".into())
            .or_default()
            .insert("ore".into(), json!({ "price": 3 }));

        let read = read_scopes(&[grant("nation:a.*")]);
        let s = slice(&w, &read);

        assert_eq!(s["nation"].len(), 1);
        assert_eq!(s["nation"]["a"], json!({ "gold": 100 }));
        // `market` matched nothing: the namespace is dropped, not left empty.
        assert!(!s.contains_key("market"));
    }

    #[test]
    fn a_leaf_grant_does_not_leak_its_siblings() {
        let mut w = StatePool::new();
        w.entry("market".into())
            .or_default()
            .insert("ore".into(), json!({ "price": 3, "stock": 99 }));

        let s = slice(&w, &read_scopes(&[grant("market:ore.price")]));
        assert_eq!(s["market"]["ore"], json!({ "price": 3 }));
    }

    #[test]
    fn dotted_names_read_through_scoped_grants() {
        let mut w = StatePool::new();
        w.entry("nation".into())
            .or_default()
            .insert("a".into(), json!({ "gold": 100 }));

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
        agents.insert("h".into(), json!({ "stamina": 3 }));
        agents.insert("rival".into(), json!({ "stamina": 9 }));
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
    fn self_alias_lifts_the_own_subtree() {
        let a = agent(vec![], None);
        let mut s = slice(&agents_world(), &read_scopes_of(&a));
        inject_self_alias(&a, &mut s);
        assert_eq!(s["self"]["stamina"], json!(3));
        // Only the own subtree was granted and aliased.
        assert!(!s[AGENTS_SCOPE].contains_key("rival"));
    }
}
