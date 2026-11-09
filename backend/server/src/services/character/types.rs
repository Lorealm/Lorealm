use crate::util::double_option;
use lorealm_storage::models::{Character, VersionedSaveMode};
use serde::{Deserialize, Serialize};

pub(super) struct EditableCharacter {
    pub character: Character,
    pub save_mode: VersionedSaveMode,
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
    pub template_injection: Option<String>,
}

/// Partial patch over `CharacterMetadata`'s editable scalar fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMetadata {
    pub creator: Option<String>,
    pub creator_notes: Option<String>,
    pub card_version: Option<String>,

    /// Three-state: absent => leave; null => clear to None; "x" => Some("x").
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "double_option"
    )]
    pub display_name: Option<Option<String>>,
}

/// The kinds of resource a character can carry with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentKind {
    Lorebook,
    Tool,
    Widget,
}

impl AttachmentKind {
    pub fn ids_mut(
        self,
        character: &mut lorealm_storage::models::Character,
    ) -> &mut Vec<lorealm_storage::models::Snowflake> {
        match self {
            Self::Lorebook => &mut character.lorebook_ids,
            Self::Tool => &mut character.tool_ids,
            Self::Widget => &mut character.widget_ids,
        }
    }

    pub fn target(self) -> crate::errors::TargetType {
        match self {
            Self::Lorebook => crate::errors::TargetType::Lorebook,
            Self::Tool => crate::errors::TargetType::UserTool,
            Self::Widget => crate::errors::TargetType::UserWidget,
        }
    }
}
