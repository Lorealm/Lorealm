use std::fmt::Display;

use eldoria_storage::{error::StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::CardParseError;

#[derive(Debug)]
pub enum TargetType {
    CharacterVersion,
    Character,
    Greeting,
    ExampleMessage,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::CharacterVersion => write!(f, "character_version"),
            TargetType::Character => write!(f, "avatar"),
            TargetType::Greeting => write!(f, "greeting"),
            TargetType::ExampleMessage => write!(f, "example_message"),
        }
    }
}

#[derive(Error, Debug)]
pub enum CharacterServiceError {
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
    #[error("permission denied")]
    PermissionDenied,
}

impl CharacterServiceError {
    pub fn client_message(&self) -> &'static str {
        match self {
            CharacterServiceError::Parse(_) => "Failed parsing character card.",
            CharacterServiceError::Storage(_) => "Unknown error.",
            CharacterServiceError::Deserialization(_error) => {
                "Failed to deserialize character card."
            }
            CharacterServiceError::UnsupportedFormat(_) => "Unsupported character card format.",
            CharacterServiceError::NotFound(_, _) | CharacterServiceError::PermissionDenied => {
                "Character either doesn't exist or you lack permission to access it."
            }
        }
    }
}
