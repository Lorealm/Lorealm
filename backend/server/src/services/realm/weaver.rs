//! Grant-scoped weaver utilities shared by every realm turn path. An agent
//! only sees and writes the state its grants cover. Holds scope projection,
//! the gated weaver run (scripts and tool actions), and run-condition
//! evaluation.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use lorealm_storage::models::{AgentAccess, AgentGrant, Param, ScopePattern, StateDelta};
use weaver_lang::{CompiledExpr, CompiledTemplate, Value as WeaverValue};

use crate::{
    errors::RealmError,
    services::{
        state::{StatePool, StateService},
        tools::{ToolContext, build_tool_registry},
    },
    util::weaver::json_to_weaver,
};

/// The scopes an agent may read: every grant grants read.
pub(super) fn read_scopes(grants: &[AgentGrant]) -> Vec<ScopePattern> {
    grants
        .iter()
        .map(|g| ScopePattern::parse(&g.scope))
        .collect()
}

/// The scopes an agent may write: only `ReadWrite` grants.
pub(super) fn write_scopes(grants: &[AgentGrant]) -> Vec<ScopePattern> {
    grants
        .iter()
        .filter(|g| matches!(g.access, AgentAccess::ReadWrite))
        .map(|g| ScopePattern::parse(&g.scope))
        .collect()
}

/// The subpool an agent may read: every variable of `world` matching any of
/// its read scopes. Namespaces left empty by the filter are dropped.
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

/// Compile and run weaver `source` against the agent's grant-filtered slice of
/// `world`, returning its output and state delta. Writes are gated by the
/// agent's `ReadWrite` grants. `params`/`args` seed the `cmd:` namespace (a
/// tool action's declared parameters; both empty for a script agent).
pub(super) async fn run_scoped(
    state: &StateService,
    source: &str,
    params: &[Param],
    args: serde_json::Value,
    grants: &[AgentGrant],
    world: &StatePool,
) -> Result<(String, StateDelta), RealmError> {
    let before = slice(world, &read_scopes(grants));

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
    let mut ctx = ToolContext::new(cmd, Arc::clone(&pool))
        .with_write_scopes(Some(Arc::new(write_scopes(grants))));

    let output = body
        .evaluate(&mut ctx, &build_tool_registry())
        .map_err(|e| RealmError::ActionFailed(e.to_string()))?;

    let delta = {
        let after = pool.lock().unwrap();
        state.diff(&before, &after)
    };
    Ok((output, delta))
}

/// Whether an agent should run on its turn: its optional `run_condition`
/// evaluated against its granted read slice of `world`. `None`/empty = always
/// runs. A condition may only read. A compile or eval error resolves to
/// `false` (the agent does not run).
pub(super) fn run_condition_passes(
    condition: &Option<String>,
    grants: &[AgentGrant],
    world: &StatePool,
) -> bool {
    let Some(src) = condition.as_ref().filter(|s| !s.trim().is_empty()) else {
        return true;
    };
    let Ok(expr) = CompiledExpr::compile(src) else {
        return false;
    };

    let read = slice(world, &read_scopes(grants));
    let pool = Arc::new(StdMutex::new(read));
    // Read-only: a condition is given no write scopes.
    let mut ctx =
        ToolContext::new(HashMap::new(), pool).with_write_scopes(Some(Arc::new(Vec::new())));

    match expr.evaluate(&mut ctx, &build_tool_registry()) {
        Ok(v) => v.is_truthy(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn grant(scope: &str) -> AgentGrant {
        AgentGrant {
            scope: scope.into(),
            access: AgentAccess::Read,
        }
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
        // weaver_lang 0.3.6: dotted names in `{{scope:name}}` literals.
        let mut w = StatePool::new();
        w.entry("nation".into())
            .or_default()
            .insert("a.gold".into(), json!(100));

        assert!(run_condition_passes(
            &Some("{{nation:a.gold}} > 50".into()),
            &[grant("nation:a.*")],
            &w
        ));
        // Granted only b.*: a.gold is invisible, so the read errors closed.
        assert!(!run_condition_passes(
            &Some("{{nation:a.gold}} > 50".into()),
            &[grant("nation:b.*")],
            &w
        ));
    }

    #[test]
    fn absent_or_empty_condition_always_runs() {
        assert!(run_condition_passes(&None, &[grant("counter")], &world()));
        assert!(run_condition_passes(
            &Some("   ".into()),
            &[grant("counter")],
            &world()
        ));
    }

    #[test]
    fn truthy_condition_runs_falsy_skips() {
        let g = [grant("counter")];
        assert!(run_condition_passes(
            &Some("{{counter:n}} > 5".into()),
            &g,
            &world()
        ));
        assert!(!run_condition_passes(
            &Some("{{counter:n}} > 10".into()),
            &g,
            &world()
        ));
    }

    #[test]
    fn broken_expression_errors_closed() {
        // A malformed expression never runs the agent.
        assert!(!run_condition_passes(
            &Some("{{counter:n} >".into()),
            &[grant("counter")],
            &world()
        ));
    }

    #[test]
    fn condition_reads_only_the_granted_slice() {
        // Without a grant on `counter`, its value is invisible: the missing
        // read makes `> 5` error closed to false even though the world has n=7.
        assert!(!run_condition_passes(
            &Some("{{counter:n}} > 5".into()),
            &[grant("other")],
            &world()
        ));
    }
}
