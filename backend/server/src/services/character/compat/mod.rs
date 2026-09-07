//! CCv2 compatibility layer.
//!
//! These types and functions are private to the character service module.
//! Nothing here escapes `CharacterService` — external callers only ever
//! see the domain `Character` type.
//!
//! Lorealm-specific data is carried inside CCv2's `extensions.lorealm`
//! namespace rather than a separate spec. This keeps SillyTavern
//! round-trip compatibility while letting us attach our own fields.

mod lorebook;

use std::collections::HashMap;

use chrono::Utc;
use lorealm_storage::models::{
    Character, CharacterDefinition, CharacterMetadata, ExampleDialogue, Greeting, Lorebook,
    Snowflake,
};
use serde::Deserialize;

use crate::{services::snowflake::SnowflakeService, util::BulkReplace};

pub use lorebook::ImportedLorebook;
use lorebook::{CharacterBook, convert_character_book};

#[derive(Deserialize)]
pub(super) struct CharaV2 {
    #[allow(dead_code)]
    pub spec: String,
    #[allow(dead_code)]
    pub spec_version: String,
    pub data: CharaV2Data,
}

/// Deserialize a field that may be present but `null`.
///
/// `#[serde(default)]` alone only covers an *absent* key: an explicit
/// `"mes_example": null` still reaches the field's own `Deserialize`, and
/// `String`, `Vec` and `bool` all reject it. Exporters write `null` for
/// "unset" freely, so every defaulted non-`Option` field on a card pairs
/// `default` with this.
pub(super) fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Every field is defaulted: V3 cards and the various exporters differ on
/// which of these they bother to emit, and a missing `system_prompt` is no
/// reason to reject a card.
#[derive(Deserialize)]
pub(super) struct CharaV2Data {
    #[serde(default, deserialize_with = "null_as_default")]
    pub name: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub description: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub personality: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub scenario: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub first_mes: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub mes_example: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub creator_notes: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub system_prompt: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub post_history_instructions: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub alternate_greetings: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "null_as_default")]
    pub creator: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub character_version: String,
    #[serde(default, deserialize_with = "null_as_default")]
    pub extensions: HashMap<String, serde_json::Value>,

    /// Held as a raw value and converted separately: books in the wild play
    /// fast and loose with field types, and a book we can't read must never
    /// take the character down with it.
    #[serde(default)]
    pub character_book: Option<serde_json::Value>,
}

/// Fields Lorealm writes into CCv2's `extensions.lorealm` object on export
/// and reads back on import. All fields are optional — cards missing this
/// namespace, or missing individual fields, must import cleanly.
///
/// NEVER rename or remove fields on this struct.
#[derive(Deserialize, Default)]
pub(super) struct LorealmExtension {
    pub display_name: Option<String>,
    pub alt_names: Option<Vec<String>>,
}

/// A card unpacked into the domain types it maps onto.
pub(super) struct ConvertedCard {
    pub character: Character,
    /// Present when the card embedded a usable `character_book`. The book
    /// must be persisted before the character, which already points at it.
    pub lorebook: Option<(Lorebook, ImportedLorebook)>,
}

/// Convert a parsed CCv2/V3 card into a fresh `Character`, plus the lorebook
/// its `character_book` describes.
pub(super) fn convert_from_ccv2(
    card: CharaV2,
    owner: Snowflake,
    snowflake: &SnowflakeService,
) -> ConvertedCard {
    let data = card.data;

    let lorealm_ext: LorealmExtension = data
        .extensions
        .get("lorealm")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    // Merge first_mes + alternate_greetings into a single ordered list.
    let mut greeting_texts = Vec::with_capacity(1 + data.alternate_greetings.len());
    greeting_texts.push(convert_variables(&data.first_mes));
    greeting_texts.extend(data.alternate_greetings.iter().map(convert_variables));

    let greetings = greeting_texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| Greeting {
            id: snowflake.generate(),
            order: i as u32,
            text: convert_variables(&text),
        })
        .collect();

    // mes_example is <START>-delimited. Split, trim empty leading segment
    // that appears when the string begins with <START>.
    let example_messages = data
        .mes_example
        .split("<START>")
        .filter(|s| !s.trim().is_empty())
        .enumerate()
        .map(|(i, content)| ExampleDialogue {
            id: snowflake.generate(),
            order: i as u32,
            content: convert_variables(&content.to_string()),
        })
        .collect();

    // Re-serialize extensions as a single JSON value for storage.
    // The raw HashMap preserves everything, including our own lorealm key,
    // which is correct — round-trip requires it to be there on export.
    let extensions = serde_json::to_value(&data.extensions).unwrap_or(serde_json::Value::Null);

    let now = Utc::now();

    // A book that fails to deserialize is logged and dropped: the character
    // is still worth importing without it.
    let lorebook = data
        .character_book
        .and_then(|raw| match serde_json::from_value::<CharacterBook>(raw) {
            Ok(book) => Some(book),
            Err(e) => {
                tracing::warn!(
                    "character '{}' has an unreadable character_book: {e}",
                    data.name
                );
                None
            }
        })
        .and_then(|book| convert_character_book(book, &data.name, owner, snowflake));

    let character = Character {
        id: snowflake.generate(),
        user_id: owner,
        current_version_id: snowflake.generate(),
        lorebook_id: lorebook.as_ref().map(|(book, _)| book.id),
        widget_ids: Vec::new(),
        extensions,
        metadata: CharacterMetadata {
            display_name: lorealm_ext.display_name,
            alt_names: lorealm_ext.alt_names.unwrap_or_default(),
            tags: data.tags,
            creator: data.creator,
            creator_notes: data.creator_notes,
            card_version: if data.character_version.is_empty() {
                "0.0.1".to_string()
            } else {
                data.character_version
            },
            avatar_uri: None, // This is persisted seperately
            created_at: now,
            updated_at: now,
        },
        definition: CharacterDefinition {
            name: data.name,
            description: convert_variables(&data.description),
            personality: convert_variables(&data.personality),
            scenario: convert_variables(&data.scenario),
            system_prompt: convert_variables(&data.system_prompt),
            post_history_instructions: convert_variables(&data.post_history_instructions),
            example_messages,
            greetings,
        },
    };

    ConvertedCard {
        character,
        lorebook,
    }
}

fn convert_variables(content: &String) -> String {
    // TODO: consider extending with more common macros:
    // https://docs.sillytavern.app/usage/core-concepts/macros/#common-macros-by-category
    BulkReplace::new(content)
        .register("{{user}}", "{{user:name}}")
        .register("{{char}}", "{{char:name}}")
        .execute()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_survives_explicit_nulls() {
        let card: CharaV2 = serde_json::from_value(serde_json::json!({
            "spec": "chara_card_v2",
            "spec_version": "2.0",
            "data": {
                "name": "Nyx",
                "description": null,
                "personality": null,
                "scenario": null,
                "first_mes": null,
                "mes_example": null,
                "creator": null,
                "creator_notes": null,
                "system_prompt": null,
                "post_history_instructions": null,
                "alternate_greetings": null,
                "tags": null,
                "character_version": null,
                "extensions": null,
                "character_book": null
            }
        }))
        .expect("nulls must not fail the card");

        assert_eq!(card.data.name, "Nyx");
        assert_eq!(card.data.description, "");
        assert!(card.data.alternate_greetings.is_empty());
        assert!(card.data.tags.is_empty());
        assert!(card.data.extensions.is_empty());
        assert!(card.data.character_book.is_none());

        let sf = SnowflakeService::new(1);
        let converted = convert_from_ccv2(card, Snowflake(9), &sf);
        assert_eq!(converted.character.definition.name, "Nyx");
        assert_eq!(converted.character.metadata.card_version, "0.0.1");
        assert!(converted.lorebook.is_none());
    }
}
