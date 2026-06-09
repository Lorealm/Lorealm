//! CCv2 compatibility layer.
//!
//! These types and functions are private to the character service module.
//! Nothing here escapes `CharacterService` — external callers only ever
//! see the domain `Character` type.
//!
//! Eldoria-specific data is carried inside CCv2's `extensions.eldoria`
//! namespace rather than a separate spec. This keeps SillyTavern
//! round-trip compatibility while letting us attach our own fields.

use std::collections::HashMap;

use chrono::Utc;
use eldoria_storage::models::{
    Character, CharacterDefinition, CharacterMetadata, ExampleDialogue, Greeting, Snowflake,
};
use serde::Deserialize;

use crate::{services::snowflake::SnowflakeService, util::BulkReplace};

#[derive(Deserialize)]
pub(super) struct CharaV2 {
    #[allow(dead_code)]
    pub spec: String,
    #[allow(dead_code)]
    pub spec_version: String,
    pub data: CharaV2Data,
}

#[derive(Deserialize)]
pub(super) struct CharaV2Data {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub first_mes: String,
    pub mes_example: String,
    pub creator_notes: String,
    pub system_prompt: String,
    pub post_history_instructions: String,
    pub alternate_greetings: Vec<String>,
    pub tags: Vec<String>,
    pub creator: String,
    pub character_version: String,
    pub extensions: HashMap<String, serde_json::Value>,
    // TODO(lorebook): pub character_book: Option<CharaV2Lorebook>
}

/// Fields Eldoria writes into CCv2's `extensions.eldoria` object on export
/// and reads back on import. All fields are optional — cards missing this
/// namespace, or missing individual fields, must import cleanly.
///
/// NEVER rename or remove fields on this struct.
#[derive(Deserialize, Default)]
pub(super) struct EldoriaExtension {
    pub display_name: Option<String>,
    pub alt_names: Option<Vec<String>>,
}

/// Convert a parsed CCv2 card into a fresh `Character`.
pub(super) fn convert_from_ccv2(
    card: CharaV2,
    owner: Snowflake,
    snowflake: &SnowflakeService,
) -> Character {
    let data = card.data;

    let eldoria_ext: EldoriaExtension = data
        .extensions
        .get("eldoria")
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
    // The raw HashMap preserves everything, including our own eldoria key,
    // which is correct — round-trip requires it to be there on export.
    let extensions = serde_json::to_value(&data.extensions).unwrap_or(serde_json::Value::Null);

    let now = Utc::now();

    Character {
        id: snowflake.generate(),
        user_id: owner,
        current_version_id: snowflake.generate(),
        lorebook_id: None, // TODO(lorebook): convert character_book
        extensions,
        metadata: CharacterMetadata {
            display_name: eldoria_ext.display_name,
            alt_names: eldoria_ext.alt_names.unwrap_or_default(),
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
