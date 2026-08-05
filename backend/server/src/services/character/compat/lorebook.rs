//! `character_book` → Lorealm lorebook conversion.
//!
//! Character cards (CCv2 and V3 alike) may embed a SillyTavern-style
//! `character_book`. This module turns one into a storage `Lorebook` whose
//! entries are real `.weaver` sources.
//!
//! This conversion process is currently lossy while we work on expanding the
//! general infrastructure of the app. Unsupported ST macros such as {{getvar::name}},
//! {{random::a::b::c}}, etc. will be replaced by equivalent macros/reserved variables
//! down the road.
//!
//! Refer to https://docs.sillytavern.app/usage/core-concepts/macros/#common-macros-by-category for more.
//!
//! NOTE: Nothing here is allowed to fail the character import: a book that cannot
//! be converted is reported and skipped, and the character still lands.

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use context_weaver::{EntryMeta, LorebookConfig, Slot};
use lorealm_storage::models::{Lorebook, LorebookEntry, Snowflake};
use serde::Deserialize;

use crate::services::snowflake::SnowflakeService;

#[derive(Debug, Deserialize)]
pub(super) struct CharacterBook {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub scan_depth: Option<usize>,
    #[serde(default)]
    pub token_budget: Option<usize>,
    // ContextWeaver does not have an option to outright disable recursion yet
    // so this field is never used
    #[serde(default)]
    pub recursive_scanning: Option<bool>,
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub entries: Vec<BookEntry>,
}

#[derive(Debug, Deserialize)]
pub(super) struct BookEntry {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub keys: Vec<String>,
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub secondary_keys: Vec<serde_json::Value>,
    #[serde(default, deserialize_with = "super::null_as_default")]
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
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub constant: bool,
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub selective: bool,
    /// `"before_char"`, `"after_char"`, `"at_depth"`, … Absent on many cards,
    /// in which case `extensions.position` carries the numeric form.
    #[serde(default)]
    pub position: Option<String>,
    #[serde(default, deserialize_with = "super::null_as_default")]
    pub extensions: EntryExtensions,
}

/// The ST `extensions` blob on an entry. Only the fields weaver can act on
/// are named. Unsupported fields are collceted in `extra` and end up in the entry's
/// frontmatter extensions.
#[derive(Debug, Default, Deserialize)]
pub(super) struct EntryExtensions {
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

/// `enabled`'s null-tolerant reader. Unlike the other flags it defaults to
/// `true`, so it cannot share `super::null_as_default`.
fn null_as_true<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(deserializer)?.unwrap_or(true))
}

/// What a book conversion produced. Surfaced to the client so the import
/// toast can say something truthful about entry counts.
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
///
/// `character_name` only supplies a fallback title for books that ship
/// without a name of their own.
pub(super) fn convert_character_book(
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
    // Per-entry in ST, book-level in ContextWeaver. Take the majority vote so the
    // common "every entry agrees" case is preserved exactly.
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

    // A book whose every entry was unusable is not worth creating.
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

/// Convert one ST entry into its `(entry_id, .weaver source)` pair, or `None`
/// if the result would not parse.
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

    // ST knobs weaver has no field for. Preserved so nothing is lost on the
    // way in, and so a future feature can pick them up.
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

    let meta = EntryMeta {
        id,
        name: display_name,
        keywords: entry.keys,
        regex: Vec::new(),
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

    // The editor and `LorebookService::update_entry` both refuse sources that
    // don't parse; refuse them here too rather than persisting a landmine.
    match context_weaver::Entry::parse(&source, None) {
        Ok(_) => Some((meta.id, source)),
        Err(e) => {
            tracing::warn!("dropping imported lorebook entry '{}': {e}", meta.name);
            None
        }
    }
}

/// Converts ST position semantics to ContextWeaver slots
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
///
/// weaver-lang has no escape for a literal `{{`, so every `{{…}}` span in the
/// source is either translated or removed:
///
///   * `{{user}}` / `{{char}}` -> their weaver spellings
///   * spans that are already weaver host variables → left alone
///   * everything else (`{{random:a,b}}`, `{{getvar::x}}`, …) -> (currently) dropped
///
/// Other construct openers (`@[`, `$[`, `[[`, `{#`, `<trigger`) are broken
/// with a space. They are vanishingly rare in prose, and a stray space beats
/// losing the entry.
fn sanitize_body(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];

        let Some(end) = after.find("}}") else {
            // Unbalanced `{{` — nothing after it can be a construct.
            rest = after;
            continue;
        };

        let inner = after[..end].trim();
        match inner.to_ascii_lowercase().as_str() {
            "user" => out.push_str("{{user:name}}"),
            "char" | "bot" => out.push_str("{{char:name}}"),
            // Already-weaver spellings, e.g. re-importing a card Lorealm wrote.
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

/// Slugify a display name into a weaver entry id, keeping it unique within
/// the book (the storage layer enforces `UNIQUE (lorebook_id, entry_id)`).
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

/// ST stores case sensitivity per entry; weaver stores it per book.
fn majority_case_sensitive(entries: &[BookEntry]) -> bool {
    let sensitive = entries
        .iter()
        .filter(|e| e.case_sensitive.unwrap_or(false))
        .count();
    sensitive * 2 > entries.len()
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
        // No `}}` to close it — must not panic or loop forever.
        assert_eq!(sanitize_body("a {{ unclosed b"), "a  unclosed b");
    }

    #[test]
    fn breaks_other_construct_openers() {
        assert_eq!(sanitize_body("see [[the tower]]"), "see [[the tower]]");
    }

    #[test]
    fn sanitized_bodies_compile() {
        // The bodies weaver rejected in isolation must all survive the pass.
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

        // Ids are deduped, and every stored source round-trips through the
        // same parser the editor uses.
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
}
