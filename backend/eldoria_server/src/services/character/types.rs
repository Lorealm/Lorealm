use serde::{Deserialize, Serialize};

use eldoria_storage::models::{Character, Snowflake, VersionedSaveMode};

pub(super) struct EditableCharacter{
    pub character: Character,
    pub save_mode: VersionedSaveMode
}

/// Partial patch over `CharacterDefinition`'s scalar fields.
/// `None` = field absent from this patch, leave it as-is.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditDefinition {
    pub name: Option<String>,
    pub description: Option<String>,
    pub personality: Option<String>,
    pub scenario: Option<String>,
    pub system_prompt: Option<String>,
    pub post_history_instructions: Option<String>,
}

/// Partial patch over `CharacterMetadata`'s editable scalar fields.
///
/// `avatar_uri` is intentionally absent — it's owned by the image endpoint.
/// `created_at` / `updated_at` are server-managed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMetadata {
    pub creator: Option<String>,
    pub creator_notes: Option<String>,
    pub card_version: Option<String>,

    /// Three-state: absent => leave; null => clear to None; "x" => Some("x").
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "double_option")]
    pub display_name: Option<Option<String>>,
}

/// Lets a nullable field distinguish "absent" from "explicit null".
/// Without this, `Option<String>` collapses both to `None`.
fn double_option<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Ok(Some(Option::deserialize(deserializer)?))
}