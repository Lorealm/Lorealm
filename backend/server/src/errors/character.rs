use lorealm_storage::{error::StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{CardParseError, TargetType};

#[derive(Error, Debug)]
pub enum CharacterError {
    #[error("failed to parse character card: {0}")]
    Parse(#[from] CardParseError),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("failed to deserialize card JSON: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("unsupported card format: '{0}' (expected chara_card_v2)")]
    UnsupportedFormat(String),
    #[error("{0} {1} does not exist")]
    NotFound(TargetType, Snowflake),
    #[error("character not owned: {0}")]
    NotOwned(Snowflake),
    #[error("permission denied")]
    PermissionDenied,
    #[error("unsupported media type")]
    UnsupportedMediaType,
}

impl CharacterError {
    pub fn client_message(&self) -> &'static str {
        match self {
            CharacterError::Parse(_) => "Failed parsing character card.",
            CharacterError::Storage(_) => "Unknown error.",
            CharacterError::Deserialization(_error) => "Failed to deserialize character card.",
            CharacterError::UnsupportedFormat(_) => "Unsupported character card format.",
            CharacterError::NotFound(_, _)
            | CharacterError::PermissionDenied
            | CharacterError::NotOwned(_) => {
                "Character either doesn't exist or you lack permission to access it."
            }
            CharacterError::UnsupportedMediaType => "unsupported media type",
        }
    }
}
