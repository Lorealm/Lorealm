use serde::{Deserialize, Serialize};
use crate::util::double_option;
use lorealm_storage::models::{Character, VersionedSaveMode};

pub(super) struct EditableCharacter{
    pub character: Character,
    pub save_mode: VersionedSaveMode
}

/// Partial patch over `CharacterDefinition`'s scalar fields.
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMetadata {
    pub creator: Option<String>,
    pub creator_notes: Option<String>,
    pub card_version: Option<String>,

    /// Three-state: absent => leave; null => clear to None; "x" => Some("x").
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "double_option")]
    pub display_name: Option<Option<String>>,
}