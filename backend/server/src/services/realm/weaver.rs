use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use lorealm_storage::models::{
    AGENTS_SCOPE, AgentGrant, AgentKind, Param, RealmAgent, Scenario, ScopePattern, StateDelta,
};
use serde_json::json;

use crate::{
    errors::RealmError,
    services::{
        state::StatePool,
        tools::{Actor, EmittedEvent, RealmView},
        weaver::{
            WeaverError, WeaverHost, WeaverProfile, bind_json_args, compile_expr, compile_template,
        },
    },
};

/// What the engine knows about the turn being planned, for `turn:`.
pub(super) struct TurnInfo<'a> {
    /// Agent turns already committed on the active path.
    pub index: usize,
    /// The cast member whose turn produced the last message, and what it said.
    pub last: Option<(&'a RealmAgent, &'a str)>,
}

impl TurnInfo<'_> {
    #[cfg(test)]
    pub(super) fn opening() -> Self {
        Self {
            index: 0,
            last: None,
        }
    }
}

pub(super) fn realm_view(
    agents: &[RealmAgent],
    turn: &TurnInfo,
    scenario: Option<&Scenario>,
) -> RealmView {
    let roster: Vec<&RealmAgent> = agents.iter().collect();

    let group = |of: &[&RealmAgent]| {
        json!({
            "count": of.len(),
            "handles": of.iter().map(|a| &a.handle).collect::<Vec<_>>(),
            "names": of.iter().map(|a| &a.name).collect::<Vec<_>>(),
        })
    };

    let mut cast = HashMap::new();
    let all = group(&roster);
    cast.insert("count".to_string(), all["count"].clone());
    cast.insert("handles".to_string(), all["handles"].clone());
    cast.insert("names".to_string(), all["names"].clone());

    for kind in [AgentKind::Llm, AgentKind::Player, AgentKind::Script] {
        let of_kind: Vec<&RealmAgent> = roster.iter().copied().filter(|a| a.kind == kind).collect();
        cast.insert(kind.as_str().to_string(), group(&of_kind));
    }

    cast.insert(
        "agents".to_string(),
        roster
            .iter()
            .enumerate()
            .map(|(index, a)| {
                (
                    a.handle.clone(),
                    json!({
                        "name": a.name,
                        "kind": a.kind.as_str(),
                        "turn_order": index,
                    }),
                )
            })
            .collect::<serde_json::Map<_, _>>()
            .into(),
    );

    let mut this_turn = HashMap::from([("index".to_string(), json!(turn.index))]);
    if let Some((agent, said)) = turn.last {
        this_turn.insert(
            "last".to_string(),
            json!({
                "handle": agent.handle,
                "name": agent.name,
                "kind": agent.kind.as_str(),
                "said": said,
            }),
        );
    }

    // Absent when the realm has no openings
    let scenario = scenario
        .map(|s| {
            HashMap::from([
                ("id".to_string(), json!(s.id)),
                ("name".to_string(), json!(s.name)),
            ])
        })
        .unwrap_or_default();

    RealmView::new(HashMap::from([
        ("cast".to_string(), cast),
        ("turn".to_string(), this_turn),
        ("scenario".to_string(), scenario),
    ]))
}

/// The scopes an agent may read
pub(super) fn read_scopes(grants: &[AgentGrant]) -> Vec<ScopePattern> {
    grants
        .iter()
        .map(|g| ScopePattern::parse(&g.channel))
        .collect()
}

pub(super) fn read_scopes_of(agent: &RealmAgent) -> Vec<ScopePattern> {
    let mut scopes = read_scopes(&agent.grants);
    scopes.push(agent.own_scope());
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
    view: &RealmView,
) -> Result<(String, StateDelta, Vec<EmittedEvent>), RealmError> {
    // Declared params land in the `cmd:` namespace the source reads. Scripts
    // declare none, so this is a no-op for them.
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmScript)
        .args(bind_json_args(params, &args).map_err(action_failed)?)
        .actor(Some(actor_of(agent)))
        .realm(view.clone())
        .events(Default::default())
        .pool(Arc::new(StdMutex::new(world.clone())))
        .build();

    let outcome = host
        .evaluate(compile_template(source).map_err(action_failed)?)
        .await
        .map_err(action_failed)?;

    Ok((outcome.to_output_string(), outcome.delta, outcome.events))
}

fn action_failed(e: WeaverError) -> RealmError {
    RealmError::ActionFailed(e.to_string())
}

/// Whether an agent should run on its turn
pub(super) fn run_condition_passes(
    agent: &RealmAgent,
    world: &StatePool,
    view: &RealmView,
) -> bool {
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
        .realm(view.clone())
        .pool(Arc::new(StdMutex::new(read)))
        .build();

    host.evaluate_inline(expr)
        .map(|outcome| outcome.is_truthy())
        .unwrap_or(false)
}

/// Run a [`Director::Script`] body: the handles it names, in the order it
/// named them.
///
/// [`Director::Script`]: lorealm_storage::models::Director::Script
pub(super) fn run_director(
    source: &str,
    world: &StatePool,
    view: &RealmView,
) -> Result<Vec<String>, RealmError> {
    let body = compile_template(source).map_err(director_failed)?;
    let host = WeaverHost::builder()
        .profile(WeaverProfile::RealmDirector)
        .realm(view.clone())
        .pool(Arc::new(StdMutex::new(world.clone())))
        .build();

    let value = host
        .evaluate_inline(body)
        .map_err(director_failed)?
        .to_json()
        .map_err(|e| RealmError::DirectorFailed(e.to_string()))?;

    let named = match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::String(handle) => vec![handle],
        serde_json::Value::Array(items) => items
            .into_iter()
            .map(|v| match v {
                serde_json::Value::String(handle) => Ok(handle),
                other => Err(RealmError::DirectorFailed(format!(
                    "expected a handle, got {other}"
                ))),
            })
            .collect::<Result<_, _>>()?,
        other => {
            return Err(RealmError::DirectorFailed(format!(
                "expected a handle or an array of handles, got {other}"
            )));
        }
    };

    // Rendered text carries the body's whitespace; a blank render is the
    // legal way to say nobody acts.
    Ok(named
        .into_iter()
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .collect())
}

fn director_failed(e: WeaverError) -> RealmError {
    RealmError::DirectorFailed(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::tools::ToolContext;
    use lorealm_storage::models::AgentAccess;
    use serde_json::json;
    use weaver_lang::EvalContext;

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
            handle: "h".into(),
            name: "tester".into(),
            kind: lorealm_storage::models::AgentKind::Script,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            user_id: None,
            persona_id: None,
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
        let a = agent(grants, Some(condition));
        let view = realm_view(std::slice::from_ref(&a), &TurnInfo::opening(), None);
        run_condition_passes(&a, world, &view)
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
            &world(),
            &RealmView::default(),
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

    /// A player, one script agent and two LLMs. The shape of a group chat.
    fn troupe() -> Vec<RealmAgent> {
        let mut agents = Vec::new();
        for (handle, kind) in [
            ("you", lorealm_storage::models::AgentKind::Player),
            ("clock", lorealm_storage::models::AgentKind::Script),
            ("alice", lorealm_storage::models::AgentKind::Llm),
            ("bob", lorealm_storage::models::AgentKind::Llm),
        ] {
            let mut a = agent(vec![], None);
            a.handle = handle.into();
            a.name = handle.to_uppercase();
            a.kind = kind;
            agents.push(a);
        }
        agents
    }

    #[test]
    fn the_roster_counts_the_cast_by_kind() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let read = |scope: &str, name: &str| {
            ToolContext::new(HashMap::new(), Default::default())
                .with_realm(Some(view.clone()))
                .resolve_variable(scope, name)
                .unwrap()
        };

        assert_eq!(read("cast", "count"), Some(weaver_lang::Value::Number(4.0)));
        assert_eq!(
            read("cast", "llm.count"),
            Some(weaver_lang::Value::Number(2.0))
        );
        assert_eq!(
            read("cast", "llm.handles"),
            Some(weaver_lang::Value::Array(vec![
                weaver_lang::Value::String("alice".into()),
                weaver_lang::Value::String("bob".into()),
            ]))
        );
        assert_eq!(
            read("cast", "agents.bob.kind"),
            Some(weaver_lang::Value::String("llm".into()))
        );
        assert_eq!(read("turn", "index"), Some(weaver_lang::Value::Number(0.0)));
        // Nothing has acted yet, so there is no last turn to read.
        assert_eq!(read("turn", "last.handle"), None);
    }

    #[test]
    fn every_cast_member_is_addressable_by_handle() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let ctx = ToolContext::new(HashMap::new(), Default::default()).with_realm(Some(view));

        for agent in &agents {
            assert_eq!(
                ctx.resolve_variable("cast", &format!("agents.{}.name", agent.handle))
                    .unwrap(),
                Some(weaver_lang::Value::String(agent.name.clone())),
                "{} is missing from the roster",
                agent.handle
            );
        }
        assert_eq!(
            ctx.resolve_variable("cast", "count").unwrap(),
            Some(weaver_lang::Value::Number(agents.len() as f64))
        );
    }

    #[test]
    fn the_picked_scenario_is_readable_and_gates_a_turn_zero_script() {
        let scenario = lorealm_storage::models::Scenario {
            id: lorealm_storage::models::Snowflake(70),
            name: "Spring".into(),
            description: String::new(),
            opening: None,
        };
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), Some(&scenario));
        let ctx = ToolContext::new(HashMap::new(), Default::default()).with_realm(Some(view));

        assert_eq!(
            ctx.resolve_variable("scenario", "name").unwrap(),
            Some(weaver_lang::Value::String("Spring".into()))
        );
        assert_eq!(
            ctx.resolve_variable("scenario", "id").unwrap(),
            Some(weaver_lang::Value::String("70".into()))
        );
    }

    #[test]
    fn a_realm_without_an_opening_has_no_scenario_to_read() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let ctx = ToolContext::new(HashMap::new(), Default::default()).with_realm(Some(view));

        assert_eq!(ctx.resolve_variable("scenario", "name").unwrap(), None);
    }

    #[test]
    fn a_run_condition_reads_the_cast_around_it() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let mut alice = agents[2].clone();
        alice.run_condition = Some("{{cast:llm.count}} == 2".into());

        assert!(run_condition_passes(&alice, &StatePool::new(), &view));
    }

    #[test]
    fn a_director_names_a_handle() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let world = StatePool::new();

        assert_eq!(
            run_director("  alice\n", &world, &view).unwrap(),
            vec!["alice".to_string()]
        );
        assert_eq!(
            run_director(
                "{# if {{cast:llm.count}} > 1 #}bob{# else #}alice{# endif #}",
                &world,
                &view
            )
            .unwrap(),
            vec!["bob".to_string()]
        );
        assert_eq!(
            run_director(r#"{# return ["alice", "bob"] #}"#, &world, &view).unwrap(),
            vec!["alice".to_string(), "bob".to_string()]
        );
        // Nobody acts.
        assert!(run_director("   ", &world, &view).unwrap().is_empty());
        // A number is not a handle.
        assert!(run_director("{# return 1 + 1 #}", &world, &view).is_err());
        // A director may not write.
        assert!(run_director(r#"$[set_var("a:b", 1)]"#, &world, &view).is_err());
    }

    #[tokio::test]
    async fn a_script_agent_reads_the_turn_it_runs_on() {
        let agents = troupe();
        let view = realm_view(&agents, &TurnInfo::opening(), None);
        let (_, delta, _) = run_scoped(
            r#"$[set_var("run:idx", @[math.mod(a: {{turn:index}}, b: {{cast:llm.count}})])]"#,
            &[],
            json!({}),
            &agents[1],
            &StatePool::new(),
            &view,
        )
        .await
        .unwrap();

        assert_eq!(delta.writes["run"]["idx"], json!(0));
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
