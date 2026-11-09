use context_weaver::{Entry, LorebookConfig, Slot};
use lorealm_storage::models::Lorebook;
use serde_json::{Map, Value, json};

use super::tokens::weaver_to_sillytavern;

pub struct ConvertedBook {
    pub book: Value,
    /// Entries using Weaver that SillyTavern can't run (in the text, or as a
    /// condition or regex keys).
    pub non_portable_entries: usize,
    /// Entries left out because they didn't parse.
    pub skipped: usize,
}

pub fn to_character_book(lorebook: &Lorebook) -> ConvertedBook {
    let config: LorebookConfig = serde_yaml::from_str(&lorebook.config_yaml).unwrap_or_default();

    let mut entries = Vec::with_capacity(lorebook.entries.len());
    let mut non_portable_entries = 0;
    let mut skipped = 0;

    for stored in &lorebook.entries {
        let Ok(entry) = Entry::parse(&stored.source, None) else {
            skipped += 1;
            continue;
        };
        let meta = &entry.meta;
        let body = weaver_to_sillytavern(&entry.source_body);
        if !body.portable || !meta.regex.is_empty() || meta.condition.is_some() {
            non_portable_entries += 1;
        }

        let (position_name, position, depth) = match meta.slot {
            Slot::Prelude | Slot::Preamble => ("before_char", 0, 0),
            Slot::Backdrop | Slot::Setting => ("after_char", 1, 0),
            Slot::Foreground => ("after_char", 2, 0),
            Slot::Coda => ("after_char", 4, 0),
            Slot::AtDepth(depth) => ("after_char", 4, depth),
        };

        // Import stores ST fields it has no use for as `st_*` extensions;
        // hand them back under their own names.
        let mut extensions = Map::new();
        let mut secondary_keys = Vec::new();
        for (key, value) in &meta.extensions {
            let Some(name) = key.strip_prefix("st_") else {
                continue;
            };
            let Ok(value) = serde_json::to_value(value) else {
                continue;
            };
            if name == "secondary_keys" {
                secondary_keys = value.as_array().cloned().unwrap_or_default();
            } else if name != "recursive_scanning" {
                extensions.insert(name.to_owned(), value);
            }
        }
        extensions.insert("position".into(), json!(position));
        extensions.insert("depth".into(), json!(depth));
        if meta.sticky_turns > 0 {
            extensions.insert("sticky".into(), json!(meta.sticky_turns));
        }
        if meta.cooldown > 0 {
            extensions.insert("cooldown".into(), json!(meta.cooldown));
        }
        if let Some(group) = &meta.group {
            extensions.insert("group".into(), json!(group));
        }
        if let Some(scan_depth) = meta.scan_depth {
            extensions.insert("scan_depth".into(), json!(scan_depth));
        }

        entries.push(json!({
            "id": entries.len(),
            "keys": meta.keywords,
            "secondary_keys": secondary_keys,
            "selective": !secondary_keys.is_empty(),
            "content": body.text,
            "enabled": meta.enabled,
            "insertion_order": meta.insertion_order,
            "case_sensitive": config.case_sensitive_keywords,
            "priority": meta.priority,
            "name": meta.name,
            "comment": meta.name,
            "constant": meta.constant,
            "use_regex": false,
            "position": position_name,
            "extensions": extensions,
        }));
    }

    let mut book = json!({
        "name": lorebook.name,
        "description": lorebook.description,
        "scan_depth": config.default_scan_depth,
        "extensions": {},
        "entries": entries,
    });
    if let Some(budget) = config.token_budget {
        book["token_budget"] = json!(budget);
    }
    if let Some(serde_yaml::Value::Bool(recursive)) = config.extensions.get("st_recursive_scanning")
    {
        book["recursive_scanning"] = json!(recursive);
    }

    ConvertedBook {
        book,
        non_portable_entries,
        skipped,
    }
}
