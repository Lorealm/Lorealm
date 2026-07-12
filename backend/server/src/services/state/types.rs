use lorealm_storage::models::{ActivationState, Snowflake, StateDelta};
use serde::Serialize;
use std::collections::HashMap;

pub type StatePool = HashMap<String, HashMap<String, serde_json::Value>>;

#[derive(Serialize)]
pub struct ReconstructedState {
    pub persistent: StatePool,
    pub activation: HashMap<Snowflake, ActivationState>,
}

#[derive(Debug)]
pub struct CapturedState {
    pub activation: HashMap<Snowflake, ActivationState>,
    pub persistent: StateDelta,
}

pub fn apply_delta(pool: &mut StatePool, d: &StateDelta) {
    for (ns, vars) in &d.writes {
        let t = pool.entry(ns.clone()).or_default();
        for (n, v) in vars {
            t.insert(n.clone(), v.clone());
        }
    }
    for (ns, names) in &d.deletes {
        if let Some(t) = pool.get_mut(ns) {
            for n in names {
                t.remove(n);
            }
            if t.is_empty() {
                pool.remove(ns);
            }
        }
    }
}
