use std::collections::{HashMap, HashSet};

use chrono::Utc;
use context_weaver::{Entry, EntryMeta, LorebookConfig, Slot};
use lorealm_storage::models::{Lorebook, LorebookEntry, Snowflake};
use serde::Deserialize;
use serde_json::{Map, Value, json};

use super::{extension::LorebookData, tokens::weaver_to_sillytavern};
use crate::services::{character::compat::null_as_default, snowflake::SnowflakeService};

#[derive(Debug, Deserialize)]
pub(crate) struct CharacterBook {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scan_depth: Option<usize>,
    #[serde(default)]
    pub token_budget: Option<usize>,
    // ContextWeaver does not have an option to outright disable recursion
    // so this field is never used
    #[serde(default)]
    pub recursive_scanning: Option<bool>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub entries: Vec<BookEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BookEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub keys: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub secondary_keys: Vec<serde_json::Value>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub content: String,
    #[serde(default = "default_true", deserialize_with = "null_as_true")]
    pub enabled: bool,
    #[serde(default)]
    pub insertion_order: Option<i32>,
    #[serde(default)]
    pub case_sensitive: Option<bool>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub constant: bool,
    #[serde(default, deserialize_with = "null_as_default")]
    pub selective: bool,
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub extensions: EntryExtensions,
}

/// The ST `extensions` blob on an entry.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct EntryExtensions {
    #[serde(default)]
    pub depth: Option<usize>,
    #[serde(default)]
    pub position: Option<u8>,
    #[serde(default)]
    pub sticky: Option<usize>,
    #[serde(default)]
    pub cooldown: Option<usize>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub scan_depth: Option<usize>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

fn null_as_true<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(true))
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ImportedLorebook {
    pub id: Snowflake,
    pub name: String,
    /// Entries that converted cleanly.
    pub entry_count: usize,
    /// Entries dropped because their body would not compile even after
    /// sanitizing. Zero for virtually every real card.
    pub skipped: usize,
}

/// Build a storage `Lorebook` from an embedded `character_book`.
pub(crate) fn convert_character_book(
    book: CharacterBook,
    character_name: &str,
    owner_id: Snowflake,
    snowflake: &SnowflakeService,
) -> Option<(Lorebook, ImportedLorebook)> {
    let name = book
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{character_name} Lorebook"));

    let description = book.description.unwrap_or_default();

    let mut config = LorebookConfig {
        name: name.clone(),
        description: description.clone(),
        ..Default::default()
    };
    if let Some(depth) = book.scan_depth {
        config.default_scan_depth = depth;
    }
    
    config.token_budget = book.token_budget;
    config.case_sensitive_keywords = majority_case_sensitive(&book.entries);
    
    if let Some(recursive) = book.recursive_scanning {
        config.extensions.insert(
            "st_recursive_scanning".to_string(),
            serde_yaml::Value::Bool(recursive),
        );
    }

    let config_yaml = match serde_yaml::to_string(&config) {
        Ok(yaml) => yaml,
        Err(e) => {
            tracing::warn!("character book '{name}' produced unserializable config: {e}");
            return None;
        }
    };

    let lorebook_id = snowflake.generate();
    let mut used_ids: HashSet<String> = HashSet::new();
    let mut entries = Vec::with_capacity(book.entries.len());
    let mut skipped = 0usize;

    for (index, entry) in book.entries.into_iter().enumerate() {
        match convert_entry(entry, index, &mut used_ids) {
            Some((entry_id, source)) => entries.push(LorebookEntry {
                id: snowflake.generate(),
                lorebook_id,
                entry_id,
                source,
            }),
            None => skipped += 1,
        }
    }

    if entries.is_empty() {
        return None;
    }

    let now = Utc::now();
    let stats = ImportedLorebook {
        id: lorebook_id,
        name: name.clone(),
        entry_count: entries.len(),
        skipped,
    };

    Some((
        Lorebook {
            id: lorebook_id,
            owner_id,
            current_version_id: lorebook_id, // placeholder
            name,
            description,
            config_yaml,
            schema_version: 1,
            created_at: now,
            updated_at: now,
            entries,
        },
        stats,
    ))
}

fn convert_entry(
    entry: BookEntry,
    index: usize,
    used_ids: &mut HashSet<String>,
) -> Option<(String, String)> {
    let display_name = entry
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .or_else(|| {
            entry
                .comment
                .as_deref()
                .map(str::trim)
                .filter(|c| !c.is_empty())
        })
        .unwrap_or("Imported Entry")
        .to_string();

    let id = unique_id(&display_name, index, used_ids);

    let mut extensions: HashMap<String, serde_yaml::Value> = HashMap::new();

    if entry.selective && !entry.secondary_keys.is_empty() {
        let secondary: Vec<serde_yaml::Value> = entry
            .secondary_keys
            .iter()
            .filter_map(|v| v.as_str())
            .map(|s| serde_yaml::Value::String(s.to_string()))
            .collect();
        if !secondary.is_empty() {
            extensions.insert(
                "st_secondary_keys".to_string(),
                serde_yaml::Value::Sequence(secondary),
            );
        }
    }
    for (key, value) in &entry.extensions.extra {
        if let Ok(yaml) = serde_yaml::to_value(value) {
            extensions.insert(format!("st_{key}"), yaml);
        }
    }

    let (keywords, regex) = split_regex_keys(entry.keys);
    let meta = EntryMeta {
        id,
        name: display_name,
        keywords,
        regex,
        condition: None,
        scan_depth: entry.extensions.scan_depth,
        constant: entry.constant,
        priority: entry.priority.unwrap_or(100),
        slot: slot_for(&entry.position, &entry.extensions),
        fallback: Vec::new(),
        insertion_order: entry.insertion_order.unwrap_or(50),
        enabled: entry.enabled,
        sticky_turns: entry.extensions.sticky.unwrap_or(0),
        cooldown: entry.extensions.cooldown.unwrap_or(0),
        group: entry.extensions.group.filter(|g| !g.trim().is_empty()),
        tags: Vec::new(),
        extensions,
    };

    let body = sanitize_body(&entry.content);
    let frontmatter = serde_yaml::to_string(&meta).ok()?;
    let source = format!("---\n{frontmatter}---\n{body}");

    match context_weaver::Entry::parse(&source, None) {
        Ok(_) => Some((meta.id, source)),
        Err(e) => {
            tracing::warn!("dropping imported lorebook entry '{}': {e}", meta.name);
            None
        }
    }
}

fn split_regex_keys(keys: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut keywords = Vec::new();
    let mut regex = Vec::new();
    for key in keys {
        match regex_from_key(&key) {
            Some(pattern) => regex.push(pattern),
            None => keywords.push(key),
        }
    }
    (keywords, regex)
}

fn regex_from_key(key: &str) -> Option<String> {
    let (pattern, flags) = key.strip_prefix('/')?.rsplit_once('/')?;
    if pattern.is_empty() || !flags.chars().all(|c| "gimsuy".contains(c)) {
        return None;
    }
    let inline: String = flags.chars().filter(|c| "ims".contains(*c)).collect();
    let pattern = if inline.is_empty() {
        pattern.to_owned()
    } else {
        format!("(?{inline}){pattern}")
    };
    regex::Regex::new(&pattern).ok()?;
    Some(pattern)
}

fn regex_to_key(pattern: &str) -> String {
    if let Some((flags, body)) = pattern
        .strip_prefix("(?")
        .and_then(|rest| rest.split_once(')'))
        && !flags.is_empty()
        && flags.chars().all(|c| "ims".contains(c))
    {
        return format!("/{body}/{flags}");
    }
    format!("/{pattern}/")
}

fn slot_for(position: &Option<String>, ext: &EntryExtensions) -> Slot {
    let depth = ext.depth.unwrap_or(0);

    if let Some(pos) = position.as_deref() {
        return match pos {
            "before_char" => Slot::Preamble,
            "after_char" => Slot::Backdrop,
            "at_depth" => Slot::AtDepth(depth),
            // Author's-note positions sit closest to the conversation.
            "before_an" | "after_an" => Slot::Foreground,
            _ => Slot::Backdrop,
        };
    }

    match ext.position {
        Some(0) => Slot::Preamble,
        Some(1) => Slot::Backdrop,
        Some(2) | Some(3) => Slot::Foreground,
        Some(4) => Slot::AtDepth(depth),
        _ => Slot::Backdrop,
    }
}

/// Rewrite an ST entry body into something weaver will compile.
fn sanitize_body(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let Some(end) = after.find("}}") else {
            rest = after;
            continue;
        };

        let inner = after[..end].trim();
        match inner.to_ascii_lowercase().as_str() {
            "user" => out.push_str("{{user:name}}"),
            "char" | "bot" => out.push_str("{{char:name}}"),
            "user:name" | "char:name" | "realm:agents" => {
                out.push_str("{{");
                out.push_str(inner);
                out.push_str("}}");
            }
            // Unmappable ST macro. Drop it.
            _ => {}
        }

        rest = &after[end + 2..];
    }
    out.push_str(rest);

    // Legacy angle-bracket macros predating {{user}}/{{char}}.
    let out = out.replace("<USER>", "{{user:name}}");
    let out = out.replace("<BOT>", "{{char:name}}");

    out
}

fn unique_id(display_name: &str, index: usize, used: &mut HashSet<String>) -> String {
    let mut slug = String::with_capacity(display_name.len());
    let mut last_underscore = false;
    for ch in display_name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_underscore = false;
        } else if !last_underscore && !slug.is_empty() {
            slug.push('_');
            last_underscore = true;
        }
    }
    let slug = slug.trim_end_matches('_').to_string();
    let base = if slug.is_empty() {
        format!("entry_{index}")
    } else {
        slug
    };

    let id = std::iter::once(base.clone())
        .chain((2..).map(|n| format!("{base}_{n}")))
        .find(|candidate| !used.contains(candidate))
        .unwrap(); // infinite iterator, always finds a free slot

    used.insert(id.clone());
    id
}

fn majority_case_sensitive(entries: &[BookEntry]) -> bool {
    let sensitive = entries
        .iter()
        .filter(|e| e.case_sensitive.unwrap_or(false))
        .count();
    sensitive * 2 > entries.len()
}

pub struct ConvertedBook {
    pub book: Value,
    pub non_portable_entries: usize,
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
        if !body.portable || meta.condition.is_some() {
            non_portable_entries += 1;
        }

        let (position_name, position, depth) = match meta.slot {
            Slot::Prelude | Slot::Preamble => ("before_char", 0, 0),
            Slot::Backdrop | Slot::Setting => ("after_char", 1, 0),
            Slot::Foreground => ("after_char", 2, 0),
            Slot::Coda => ("after_char", 4, 0),
            Slot::AtDepth(depth) => ("after_char", 4, depth),
        };

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
            "keys": meta
                .keywords
                .iter()
                .cloned()
                .chain(meta.regex.iter().map(|r| regex_to_key(r)))
                .collect::<Vec<_>>(),
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

pub fn to_world_info(lorebook: &Lorebook) -> ConvertedBook {
    let mut converted = to_character_book(lorebook);
    let mut entries = Map::new();
    let book_entries = converted.book["entries"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    for (uid, entry) in book_entries.iter().enumerate() {
        let mut fields: Map<String, Value> = entry["extensions"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, v)| (snake_to_camel(k), v.clone()))
            .collect();
        for (key, default) in [
            ("probability", json!(100)),
            ("useProbability", json!(true)),
            ("selectiveLogic", json!(0)),
        ] {
            fields.entry(key).or_insert(default);
        }
        for (key, value) in [
            ("uid", json!(uid)),
            ("key", entry["keys"].clone()),
            ("keysecondary", entry["secondary_keys"].clone()),
            ("comment", entry["comment"].clone()),
            ("content", entry["content"].clone()),
            ("constant", entry["constant"].clone()),
            ("selective", entry["selective"].clone()),
            ("order", entry["insertion_order"].clone()),
            (
                "disable",
                json!(!entry["enabled"].as_bool().unwrap_or(true)),
            ),
            ("caseSensitive", entry["case_sensitive"].clone()),
        ] {
            fields.insert(key.to_owned(), value);
        }
        entries.insert(uid.to_string(), Value::Object(fields));
    }
    converted.book = json!({
        "name": lorebook.name,
        "description": lorebook.description,
        "entries": entries,
        "extensions": { super::extension::EXTENSION_KEY: LorebookData::from_lorebook(lorebook) },
    });
    converted
}

const WORLD_FIELDS: [&str; 10] = [
    "uid",
    "key",
    "keysecondary",
    "comment",
    "content",
    "constant",
    "selective",
    "order",
    "disable",
    "caseSensitive",
];

fn world_info_to_character_book(world: &Value, name: &str) -> Value {
    let mut entries: Vec<&Map<String, Value>> = world["entries"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter_map(Value::as_object)
        .collect();
    entries.sort_by_key(|e| e.get("uid").and_then(Value::as_i64).unwrap_or(i64::MAX));

    let entries: Vec<Value> = entries
        .into_iter()
        .map(|e| {
            let extensions: Map<String, Value> = e
                .iter()
                .filter(|(k, v)| !WORLD_FIELDS.contains(&k.as_str()) && !v.is_null())
                .map(|(k, v)| (camel_to_snake(k), v.clone()))
                .collect();
            json!({
                "keys": e.get("key"),
                "secondary_keys": e.get("keysecondary"),
                "comment": e.get("comment"),
                "content": e.get("content"),
                "constant": e.get("constant"),
                "selective": e.get("selective"),
                "insertion_order": e.get("order"),
                "enabled": !e.get("disable").and_then(Value::as_bool).unwrap_or(false),
                "case_sensitive": e.get("caseSensitive"),
                "extensions": extensions,
            })
        })
        .collect();

    json!({
        "name": world.get("name").and_then(Value::as_str).unwrap_or(name),
        "description": world.get("description"),
        "entries": entries,
    })
}

pub fn import_file(
    json: &Value,
    name: &str,
    owner_id: Snowflake,
    snowflake: &SnowflakeService,
) -> Option<(Lorebook, ImportedLorebook)> {
    let native = json
        .pointer(&format!("/extensions/{}", super::extension::EXTENSION_KEY))
        .and_then(|v| serde_json::from_value::<LorebookData>(v.clone()).ok());
    if let Some(data) = native {
        let book = lorebook_from_data(&data, owner_id, snowflake)?;
        let stats = ImportedLorebook {
            id: book.id,
            name: book.name.clone(),
            entry_count: book.entries.len(),
            skipped: data.entries.len() - book.entries.len(),
        };
        return Some((book, stats));
    }

    let book = if json["entries"].is_object() {
        world_info_to_character_book(json, name)
    } else {
        json.clone()
    };
    let book: CharacterBook = serde_json::from_value(book).ok()?;
    convert_character_book(book, name, owner_id, snowflake)
}

fn snake_to_camel(key: &str) -> String {
    let mut parts = key.split('_');
    let mut out = parts.next().unwrap_or_default().to_owned();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            out.extend(first.to_uppercase());
            out.push_str(chars.as_str());
        }
    }
    out
}

fn camel_to_snake(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

pub fn lorebook_from_data(
    data: &LorebookData,
    owner_id: Snowflake,
    snowflake: &SnowflakeService,
) -> Option<Lorebook> {
    serde_yaml::from_str::<LorebookConfig>(&data.config_yaml).ok()?;
    let id = snowflake.generate();
    let entries = data
        .entries
        .iter()
        .filter(|e| Entry::parse(&e.source, None).is_ok())
        .map(|e| LorebookEntry {
            id: snowflake.generate(),
            lorebook_id: id,
            entry_id: e.entry_id.clone(),
            source: e.source.clone(),
        })
        .collect();
    let now = Utc::now();
    Some(Lorebook {
        id,
        owner_id,
        current_version_id: id, // placeholder while versioning is dormant
        name: data.name.clone(),
        description: data.description.clone(),
        config_yaml: data.config_yaml.clone(),
        schema_version: 1,
        created_at: now,
        updated_at: now,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(content: &str) -> BookEntry {
        BookEntry {
            name: Some("Test".to_string()),
            keys: vec!["test".to_string()],
            secondary_keys: Vec::new(),
            content: content.to_string(),
            enabled: true,
            insertion_order: None,
            case_sensitive: None,
            priority: None,
            comment: None,
            constant: false,
            selective: false,
            position: None,
            extensions: EntryExtensions::default(),
        }
    }

    #[test]
    fn translates_st_name_macros() {
        assert_eq!(
            sanitize_body("{{char}} greets {{user}}."),
            "{{char:name}} greets {{user:name}}."
        );
    }

    #[test]
    fn drops_unmappable_macros() {
        assert_eq!(
            sanitize_body("Roll: {{random:a,b}} and {{getvar::gold}} gold."),
            "Roll:  and  gold."
        );
    }

    #[test]
    fn keeps_existing_weaver_variables() {
        assert_eq!(sanitize_body("{{char:name}} waits"), "{{char:name}} waits");
    }

    #[test]
    fn survives_unbalanced_braces() {
        assert_eq!(sanitize_body("a {{ unclosed b"), "a  unclosed b");
    }

    #[test]
    fn breaks_other_construct_openers() {
        assert_eq!(sanitize_body("see [[the tower]]"), "see [[the tower]]");
    }

    #[test]
    fn sanitized_bodies_compile() {
        for raw in [
            "{{random:a,b}}",
            "{{getvar::foo}}",
            "{{ unclosed",
            "plain prose",
        ] {
            let body = sanitize_body(raw);
            assert!(
                weaver_lang::CompiledTemplate::compile(&body).is_ok(),
                "body {body:?} (from {raw:?}) failed to compile"
            );
        }
    }

    #[test]
    fn entry_ids_are_unique_and_slugged() {
        let mut used = HashSet::new();
        assert_eq!(
            unique_id("The Dark Forest", 0, &mut used),
            "the_dark_forest"
        );
        assert_eq!(
            unique_id("The Dark Forest", 1, &mut used),
            "the_dark_forest_2"
        );
        assert_eq!(unique_id("!!!", 7, &mut used), "entry_7");
    }

    #[test]
    fn maps_position_to_slot() {
        let ext = EntryExtensions::default();
        assert_eq!(slot_for(&Some("before_char".into()), &ext), Slot::Preamble);
        assert_eq!(slot_for(&Some("after_char".into()), &ext), Slot::Backdrop);
        assert_eq!(slot_for(&None, &ext), Slot::Backdrop);

        let at_depth = EntryExtensions {
            depth: Some(4),
            position: Some(4),
            ..Default::default()
        };
        assert_eq!(slot_for(&None, &at_depth), Slot::AtDepth(4));
    }

    #[test]
    fn converts_a_book_end_to_end() {
        let sf = SnowflakeService::new(1);
        let book = CharacterBook {
            name: Some("Nyx's Lore".to_string()),
            description: Some("world stuff".to_string()),
            scan_depth: Some(5),
            token_budget: Some(1200),
            recursive_scanning: Some(true),
            entries: vec![entry("{{char}} remembers {{random:x,y}}."), entry("more")],
        };

        let (lorebook, stats) =
            convert_character_book(book, "Nyx", Snowflake(9), &sf).expect("book should convert");

        assert_eq!(lorebook.name, "Nyx's Lore");
        assert_eq!(stats.entry_count, 2);
        assert_eq!(stats.skipped, 0);
        assert_eq!(lorebook.entries.len(), 2);

        assert_eq!(lorebook.entries[0].entry_id, "test");
        assert_eq!(lorebook.entries[1].entry_id, "test_2");
        for e in &lorebook.entries {
            let parsed = context_weaver::Entry::parse(&e.source, None).unwrap();
            assert_eq!(parsed.meta.keywords, vec!["test"]);
        }

        let config: LorebookConfig = serde_yaml::from_str(&lorebook.config_yaml).unwrap();
        assert_eq!(config.default_scan_depth, 5);
        assert_eq!(config.token_budget, Some(1200));
    }

    #[test]
    fn book_survives_explicit_nulls() {
        let raw = serde_json::json!({
            "name": "Booru Book",
            "entries": [{
                "id": 0,
                "keys": ["tower"],
                "name": "The Tower",
                "comment": null,
                "content": "{{char}} built it.",
                "enabled": true,
                "constant": null,
                "position": null,
                "priority": null,
                "selective": null,
                "use_regex": false,
                "extensions": {},
                "case_sensitive": null,
                "secondary_keys": null,
                "insertion_order": 0
            }],
            "extensions": {},
            "scan_depth": 4,
            "description": null,
            "token_budget": null,
            "creation_date": 1_700_000_000,
            "modification_date": 1_700_000_001,
            "recursive_scanning": null
        });

        let book: CharacterBook =
            serde_json::from_value(raw).expect("nulls must not fail the book");
        assert_eq!(book.entries.len(), 1);
        assert!(!book.entries[0].constant);
        assert!(!book.entries[0].selective);
        assert!(book.entries[0].secondary_keys.is_empty());

        let sf = SnowflakeService::new(1);
        let (lorebook, stats) =
            convert_character_book(book, "Nyx", Snowflake(9), &sf).expect("book should convert");
        assert_eq!(lorebook.name, "Booru Book");
        assert_eq!(stats.entry_count, 1);
        assert_eq!(lorebook.entries[0].entry_id, "the_tower");
    }

    #[test]
    fn null_containers_and_enabled_fall_back() {
        let entry: BookEntry = serde_json::from_value(serde_json::json!({
            "keys": null,
            "content": null,
            "enabled": null,
            "extensions": null,
        }))
        .expect("null containers must not fail the entry");
        assert!(entry.keys.is_empty());
        assert_eq!(entry.content, "");
        assert!(entry.enabled, "an unset `enabled` means enabled");
        assert_eq!(entry.extensions.depth, None);

        let book: CharacterBook =
            serde_json::from_value(serde_json::json!({ "name": "B", "entries": null }))
                .expect("null entries must not fail the book");
        assert!(book.entries.is_empty());
    }

    #[test]
    fn book_with_no_usable_entries_is_dropped() {
        let sf = SnowflakeService::new(1);
        let book = CharacterBook {
            name: None,
            description: None,
            scan_depth: None,
            token_budget: None,
            recursive_scanning: None,
            entries: Vec::new(),
        };
        assert!(convert_character_book(book, "Nyx", Snowflake(9), &sf).is_none());
    }

    #[test]
    fn reads_a_sillytavern_world_file() {
        let sf = SnowflakeService::new(1);
        let world = serde_json::json!({
            "entries": {
                "1": {
                    "uid": 1, "key": ["tower"], "keysecondary": [], "comment": "Tower",
                    "content": "{{user}} sees a tower.", "constant": false, "selective": true,
                    "order": 20, "position": 4, "depth": 2, "disable": true,
                    "probability": 100, "scanDepth": null, "group": ""
                },
                "0": {
                    "uid": 0, "key": ["Nyx"], "comment": "Nyx", "content": "Nyx is here.",
                    "order": 10, "position": 0, "disable": false
                }
            }
        });

        let (lorebook, stats) =
            import_file(&world, "Ruins", Snowflake(9), &sf).expect("world should import");
        assert_eq!(lorebook.name, "Ruins");
        assert_eq!(stats.entry_count, 2);

        let nyx = Entry::parse(&lorebook.entries[0].source, None).unwrap();
        assert_eq!(nyx.meta.keywords, vec!["Nyx"]);
        assert_eq!(nyx.meta.slot, Slot::Preamble);
        assert!(nyx.meta.enabled);

        let tower = Entry::parse(&lorebook.entries[1].source, None).unwrap();
        assert_eq!(tower.meta.name, "Tower");
        assert_eq!(tower.meta.insertion_order, 20);
        assert_eq!(tower.meta.slot, Slot::AtDepth(2));
        assert!(!tower.meta.enabled);
        assert!(tower.meta.extensions.contains_key("st_probability"));
    }

    #[test]
    fn world_file_round_trips() {
        let sf = SnowflakeService::new(1);
        let book = CharacterBook {
            name: Some("Nyx's Lore".to_string()),
            description: None,
            scan_depth: None,
            token_budget: None,
            recursive_scanning: None,
            entries: vec![entry("{{char}} waits."), entry("more")],
        };
        let (original, _) = convert_character_book(book, "Nyx", Snowflake(9), &sf).unwrap();

        let mut world = to_world_info(&original).book;
        assert_eq!(world["entries"]["0"]["key"], serde_json::json!(["test"]));
        assert_eq!(world["entries"]["0"]["disable"], serde_json::json!(false));

        // With Lorealm's copy, every source comes back exactly.
        let (exact, _) = import_file(&world, "x", Snowflake(9), &sf).unwrap();
        let sources = |b: &Lorebook| {
            b.entries
                .iter()
                .map(|e| e.source.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(sources(&exact), sources(&original));

        // Without it (a site stripped it), the SillyTavern fields carry it.
        world.as_object_mut().unwrap().remove("extensions");
        let (read, _) = import_file(&world, "x", Snowflake(9), &sf).unwrap();
        assert_eq!(read.name, "Nyx's Lore");
        for (a, b) in read.entries.iter().zip(&original.entries) {
            let (a, b) = (
                Entry::parse(&a.source, None).unwrap(),
                Entry::parse(&b.source, None).unwrap(),
            );
            assert_eq!(a.meta.keywords, b.meta.keywords);
            assert_eq!(a.meta.slot, b.meta.slot);
            assert_eq!(a.source_body, b.source_body);
        }
    }

    #[test]
    fn regex_keys_cross_both_ways() {
        let (keywords, regex) = split_regex_keys(
            ["tower", "/^/", "/drag(on|ons)/i", "/(?<=a)b/", "/a/b/"]
                .map(String::from)
                .to_vec(),
        );
        // Lookbehind doesn't compile in Rust, so it stays as written.
        assert_eq!(keywords, vec!["tower", "/(?<=a)b/"]);
        assert_eq!(regex, vec!["^", "(?i)drag(on|ons)", "a/b"]);

        let back: Vec<String> = regex.iter().map(|r| regex_to_key(r)).collect();
        assert_eq!(back, vec!["/^/", "/drag(on|ons)/i", "/a/b/"]);
    }
}
