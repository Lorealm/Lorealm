mod lorebook;

use std::collections::HashMap;

use chrono::Utc;
use lorealm_storage::models::{
    Character, CharacterDefinition, CharacterMetadata, ExampleDialogue, Greeting, Lorebook,
    Snowflake,
};
use serde::Deserialize;

use crate::services::{
    cards::{
        export::{split_examples, split_greetings},
        extension::{EXTENSION_KEY, LorealmCardExtension, TextDialect},
        tokens::sillytavern_to_weaver,
    },
    snowflake::SnowflakeService,
};

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
pub(super) fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

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

    /// Held as a raw value and converted separately
    #[serde(default)]
    pub character_book: Option<serde_json::Value>,
}

/// A card unpacked into the domain types it maps onto.
pub(super) struct ConvertedCard {
    pub character: Character,
    /// Present when the card embedded a usable `character_book`. The book
    /// must be persisted before the character, which already points at it.
    pub lorebook: Option<(Lorebook, ImportedLorebook)>,
    /// The card's `extensions.lorealm`; its attachments are imported separately.
    pub extension: LorealmCardExtension,
}

/// Convert a parsed CCv2/V3 card into a fresh `Character`, plus the lorebook
/// its `character_book` describes.
pub(super) fn convert_from_ccv2(
    card: CharaV2,
    owner: Snowflake,
    snowflake: &SnowflakeService,
) -> ConvertedCard {
    let data = card.data;

    let mut card_extensions = data.extensions;
    let extension: LorealmCardExtension = card_extensions
        .remove(EXTENSION_KEY)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    let read = |text: &str| match extension.dialect {
        TextDialect::Weaver => text.to_owned(),
        TextDialect::Sillytavern => sillytavern_to_weaver(text),
    };
    let fields = &extension.fields;
    let field =
        |native: &Option<String>, written: &str| native.clone().unwrap_or_else(|| read(written));

    let greeting_texts = fields.greetings.clone().unwrap_or_else(|| {
        split_greetings(&data.first_mes, &data.alternate_greetings)
            .iter()
            .map(|g| read(g))
            .collect()
    });
    let greetings = greeting_texts
        .into_iter()
        .enumerate()
        .map(|(i, text)| Greeting {
            id: snowflake.generate(),
            order: i as u32,
            text,
        })
        .collect();

    // `<START>`-separated, which is what SillyTavern recommends. There is no
    // global standard, however.
    let example_texts = fields.examples.clone().unwrap_or_else(|| {
        split_examples(&data.mes_example)
            .iter()
            .map(|e| read(e))
            .collect()
    });
    let example_messages = example_texts
        .into_iter()
        .enumerate()
        .map(|(i, content)| ExampleDialogue {
            id: snowflake.generate(),
            order: i as u32,
            content,
        })
        .collect();

    // Re-serialize extensions as a single JSON value for storage.
    // Other apps' extensions are kept for the round trip; Lorealm's own is
    // rebuilt from the character on export.
    let extensions = serde_json::to_value(&card_extensions).unwrap_or(serde_json::Value::Null);

    let now = Utc::now();

    // A book that fails to deserialize is logged and dropped
    let lorebook = data
        .character_book
        .filter(|_| extension.lorebooks.is_empty())
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
        lorebook_ids: lorebook
            .as_ref()
            .map(|(book, _)| book.id)
            .into_iter()
            .collect(),
        tool_ids: Vec::new(),
        widget_ids: Vec::new(),
        extensions,
        metadata: CharacterMetadata {
            display_name: extension.display_name.clone(),
            alt_names: extension.alt_names.clone().unwrap_or_default(),
            tags: data.tags,
            creator: data.creator,
            creator_notes: data.creator_notes,
            // Lorealm's own cards say exactly what the version was, empty or not.
            card_version: if data.character_version.is_empty() && extension.version == 0 {
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
            description: field(&fields.description, &data.description),
            personality: field(&fields.personality, &data.personality),
            scenario: field(&fields.scenario, &data.scenario),
            system_prompt: field(&fields.system_prompt, &data.system_prompt),
            post_history_instructions: field(
                &fields.post_history_instructions,
                &data.post_history_instructions,
            ),
            example_messages,
            greetings,
            template_injection: extension.template_injection.clone().unwrap_or_default(),
        },
    };

    ConvertedCard {
        character,
        lorebook,
        extension,
    }
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

    #[test]
    fn template_injection_imports_from_the_lorealm_extension() {
        let card: CharaV2 = serde_json::from_value(serde_json::json!({
            "spec": "chara_card_v2",
            "spec_version": "2.0",
            "data": {
                "name": "Nyx",
                "extensions": { "lorealm": { "template_injection": "{{template}} T" } }
            }
        }))
        .unwrap();

        let sf = SnowflakeService::new(1);
        let converted = convert_from_ccv2(card, Snowflake(9), &sf);
        assert_eq!(
            converted.character.definition.template_injection,
            "{{template}} T"
        );
    }
}
