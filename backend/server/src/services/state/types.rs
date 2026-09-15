use lorealm_storage::models::{ActivationState, Snowflake, StateDelta};
use serde::Serialize;
use serde_json::json;
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

fn set_path(slot: &mut serde_json::Value, path: &str, value: serde_json::Value) {
    if !slot.is_object() {
        *slot = serde_json::Value::Object(Default::default());
    }
    let map = slot.as_object_mut().expect("replaced with an object above");
    match path.split_once('.') {
        None => {
            map.insert(path.to_string(), value);
        }
        Some((head, rest)) => set_path(map.entry(head).or_insert_with(|| json!({})), rest, value),
    }
}

/// Remove a dotted `path` from `slot`. Returns whether anything was removed.
fn remove_path(slot: &mut serde_json::Value, path: &str) -> bool {
    let Some(map) = slot.as_object_mut() else {
        return false;
    };
    match path.split_once('.') {
        None => map.remove(path).is_some(),
        Some((head, rest)) => match map.get_mut(head) {
            Some(child) => remove_path(child, rest),
            None => false,
        },
    }
}

/// Fold one namespace write into the pool.
pub fn write_var(
    ns: &mut HashMap<String, serde_json::Value>,
    name: &str,
    value: &serde_json::Value,
) {
    match name.split_once('.') {
        None => {
            ns.insert(name.to_string(), value.clone());
        }
        Some((root, rest)) => set_path(
            ns.entry(root.to_string()).or_insert_with(|| json!({})),
            rest,
            value.clone(),
        ),
    }
}

/// Remove one name from a namespace, following a dotted path if present.
pub fn delete_var(ns: &mut HashMap<String, serde_json::Value>, name: &str) {
    match name.split_once('.') {
        None => {
            ns.remove(name);
        }
        Some((root, rest)) => {
            if let Some(slot) = ns.get_mut(root) {
                remove_path(slot, rest);
            }
        }
    }
}

pub fn apply_delta(pool: &mut StatePool, d: &StateDelta) {
    for (ns, vars) in &d.writes {
        let t = pool.entry(ns.clone()).or_default();
        for (n, v) in vars {
            write_var(t, n, v);
        }
    }
    for (ns, names) in &d.deletes {
        if let Some(t) = pool.get_mut(ns) {
            for n in names {
                delete_var(t, n);
            }
            if t.is_empty() {
                pool.remove(ns);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta(writes: &[(&str, &str, serde_json::Value)], deletes: &[(&str, &str)]) -> StateDelta {
        let mut d = StateDelta::default();
        for (ns, name, v) in writes {
            d.writes
                .entry((*ns).into())
                .or_default()
                .insert((*name).into(), v.clone());
        }
        for (ns, name) in deletes {
            d.deletes
                .entry((*ns).into())
                .or_default()
                .push((*name).into());
        }
        d
    }

    #[test]
    fn legacy_flat_names_fold_into_nested_objects() {
        let mut pool = StatePool::new();
        apply_delta(
            &mut pool,
            &delta(&[("nation", "a.army.size", json!(12))], &[]),
        );
        assert_eq!(pool["nation"]["a"], json!({ "army": { "size": 12 } }));
    }

    #[test]
    fn flat_and_nested_writes_merge_into_one_root() {
        let mut pool = StatePool::new();
        apply_delta(&mut pool, &delta(&[("nation", "a.gold", json!(1))], &[]));
        apply_delta(&mut pool, &delta(&[("nation", "a.army", json!(7))], &[]));
        assert_eq!(pool["nation"]["a"], json!({ "gold": 1, "army": 7 }));
    }

    #[test]
    fn a_nested_root_write_replaces_the_whole_subtree() {
        let mut pool = StatePool::new();
        apply_delta(&mut pool, &delta(&[("nation", "a.gold", json!(1))], &[]));
        apply_delta(
            &mut pool,
            &delta(&[("nation", "a", json!({ "army": 7 }))], &[]),
        );
        assert_eq!(pool["nation"]["a"], json!({ "army": 7 }));
    }

    #[test]
    fn deletes_follow_a_dotted_path_and_leave_siblings() {
        let mut pool = StatePool::new();
        apply_delta(
            &mut pool,
            &delta(
                &[
                    ("nation", "a.gold", json!(1)),
                    ("nation", "a.army", json!(7)),
                ],
                &[],
            ),
        );
        apply_delta(&mut pool, &delta(&[], &[("nation", "a.gold")]));
        assert_eq!(pool["nation"]["a"], json!({ "army": 7 }));
    }
}
