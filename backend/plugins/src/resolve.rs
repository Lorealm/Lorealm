use std::collections::HashSet;

use lorealm_plugin_manifest::Manifest;

/// The plugins that can load, in load order, and those that can't.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Resolution {
    pub order: Vec<String>,
    /// Handle and the reason it was left out.
    pub skipped: Vec<(String, String)>,
}

/// Resolves a user's enabled plugins, given in the user's order.
///
/// Plugins are left out, in this order, for:
/// 1. a conflict with an earlier plugin (declared by either side),
/// 2. a requirement that isn't loaded or whose version doesn't match,
/// 3. being stuck in (or behind) a load-order cycle.
///
/// The rest are sorted so `requires`, `loaded_after` and `loaded_before` hold,
/// keeping the user's order wherever the manifests don't decide it.
pub fn resolve(plugins: &[&Manifest]) -> Resolution {
    let handle = |m: &Manifest| m.plugin.handle.clone();
    let mut skipped = Vec::new();
    let mut kept: Vec<&Manifest> = Vec::new();

    for &m in plugins {
        let conflict = kept.iter().find(|k| {
            k.compatibility.conflicts.contains(&m.plugin.handle)
                || m.compatibility.conflicts.contains(&k.plugin.handle)
        });
        match conflict {
            Some(k) => skipped.push((handle(m), format!("conflicts with `{}`", k.plugin.handle))),
            None => kept.push(m),
        }
    }

    // Dropping one plugin can break another's requirement, so repeat until stable.
    loop {
        let unmet = kept.iter().enumerate().find_map(|(i, m)| {
            m.compatibility.requires.iter().find_map(|(req, range)| {
                match kept.iter().find(|k| &k.plugin.handle == req) {
                    None => Some((i, format!("requires `{req}` {range}, which is not loaded"))),
                    Some(k) if !range.matches(&k.plugin.version) => Some((
                        i,
                        format!(
                            "requires `{req}` {range}, but {} is enabled",
                            k.plugin.version
                        ),
                    )),
                    Some(_) => None,
                }
            })
        });
        let Some((i, reason)) = unmet else { break };
        skipped.push((handle(kept.remove(i)), reason));
    }

    let (order, stuck) = sort(&kept);
    if !stuck.is_empty() {
        let names: Vec<String> = stuck
            .iter()
            .map(|&i| format!("`{}`", kept[i].plugin.handle))
            .collect();
        let reason = format!("load-order cycle among {}", names.join(", "));
        skipped.extend(stuck.iter().map(|&i| (handle(kept[i]), reason.clone())));
    }

    Resolution {
        order: order.into_iter().map(|i| handle(kept[i])).collect(),
        skipped,
    }
}

/// Stable topological sort over indices into `plugins`. Returns the sorted
/// indices and those left over by a cycle.
fn sort(plugins: &[&Manifest]) -> (Vec<usize>, Vec<usize>) {
    let index = |h: &String| plugins.iter().position(|m| &m.plugin.handle == h);

    // edges[i] holds every plugin that must load before i.
    let mut edges: Vec<HashSet<usize>> = vec![HashSet::new(); plugins.len()];
    for (i, m) in plugins.iter().enumerate() {
        let c = &m.compatibility;
        for before in c.requires.keys().chain(&c.loaded_after).filter_map(index) {
            edges[i].insert(before);
        }
        for after in c.loaded_before.iter().filter_map(index) {
            edges[after].insert(i);
        }
    }

    let mut order = Vec::new();
    let mut placed = vec![false; plugins.len()];
    // The earliest plugin (in the user's order) whose predecessors are all placed.
    while let Some(next) =
        (0..plugins.len()).find(|&i| !placed[i] && edges[i].iter().all(|&b| placed[b]))
    {
        placed[next] = true;
        order.push(next);
    }
    let stuck = (0..plugins.len()).filter(|&i| !placed[i]).collect();
    (order, stuck)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A manifest for `handle` at `version`, with `compat` as its
    /// `[compatibility]` table.
    fn plugin(handle: &str, version: &str, compat: &str) -> Manifest {
        toml::from_str(&format!(
            r#"
            [plugin]
            name = "{handle}"
            handle = "{handle}"
            description = ""
            author = "Test"
            version = "{version}"
            api = "0.1.0"

            [compatibility]
            {compat}
            "#
        ))
        .unwrap()
    }

    fn run(plugins: &[Manifest]) -> Resolution {
        resolve(&plugins.iter().collect::<Vec<_>>())
    }

    fn skipped(resolution: &Resolution) -> Vec<&str> {
        resolution.skipped.iter().map(|(h, _)| h.as_str()).collect()
    }

    #[test]
    fn keeps_the_users_order_when_unconstrained() {
        let r = run(&[plugin("b", "1.0.0", ""), plugin("a", "1.0.0", "")]);

        assert_eq!(r.order, ["b", "a"]);
        assert!(r.skipped.is_empty());
    }

    #[test]
    fn requirements_load_first() {
        let r = run(&[
            plugin("app", "1.0.0", r#"requires = { lib = "^1.2" }"#),
            plugin("lib", "1.4.0", ""),
        ]);

        assert_eq!(r.order, ["lib", "app"]);
    }

    #[test]
    fn loaded_before_and_after_reorder() {
        let r = run(&[
            plugin("a", "1.0.0", r#"loaded_after = ["c"]"#),
            plugin("b", "1.0.0", ""),
            plugin("c", "1.0.0", r#"loaded_before = ["b"]"#),
        ]);

        assert_eq!(r.order, ["c", "a", "b"]);
    }

    #[test]
    fn ordering_against_a_plugin_that_isnt_enabled_is_ignored() {
        let r = run(&[plugin("a", "1.0.0", r#"loaded_after = ["missing"]"#)]);

        assert_eq!(r.order, ["a"]);
    }

    #[test]
    fn a_conflict_skips_the_later_plugin_whichever_side_declares_it() {
        for (first, second) in [(r#"conflicts = ["b"]"#, ""), ("", r#"conflicts = ["a"]"#)] {
            let r = run(&[plugin("a", "1.0.0", first), plugin("b", "1.0.0", second)]);

            assert_eq!(r.order, ["a"]);
            assert_eq!(r.skipped, [("b".into(), "conflicts with `a`".into())]);
        }
    }

    #[test]
    fn unmet_requirements_are_skipped() {
        let r = run(&[
            plugin("missing_dep", "1.0.0", r#"requires = { gone = "*" }"#),
            plugin("old_dep", "1.0.0", r#"requires = { lib = "^2" }"#),
            plugin("lib", "1.4.0", ""),
        ]);

        assert_eq!(r.order, ["lib"]);
        assert_eq!(
            r.skipped,
            [
                (
                    "missing_dep".into(),
                    "requires `gone` *, which is not loaded".into()
                ),
                (
                    "old_dep".into(),
                    "requires `lib` ^2, but 1.4.0 is enabled".into()
                ),
            ]
        );
    }

    #[test]
    fn skipping_cascades_to_dependents() {
        let r = run(&[
            plugin("a", "1.0.0", ""),
            plugin("b", "1.0.0", r#"conflicts = ["a"]"#),
            plugin("c", "1.0.0", r#"requires = { b = "*" }"#),
        ]);

        assert_eq!(r.order, ["a"]);
        assert_eq!(skipped(&r), ["b", "c"]);
    }

    #[test]
    fn a_cycle_skips_its_members_only() {
        let r = run(&[
            plugin("a", "1.0.0", r#"loaded_after = ["b"]"#),
            plugin("b", "1.0.0", r#"loaded_after = ["a"]"#),
            plugin("c", "1.0.0", ""),
        ]);

        assert_eq!(r.order, ["c"]);
        assert_eq!(skipped(&r), ["a", "b"]);
        assert_eq!(r.skipped[0].1, "load-order cycle among `a`, `b`");
    }
}
