use eldoria_storage::error::StorageError;
use thiserror::Error;

use crate::errors::CardParseError;

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
}

impl CharacterServiceError {
    pub fn client_message(&self) -> &'static str {
            match self {
                CharacterServiceError::Parse(_) => "Failed parsing character card.",
                CharacterServiceError::Storage(_) => "Unknown error.",
                CharacterServiceError::Deserialization(_error) => "Failed to deserialize character card.",
                CharacterServiceError::UnsupportedFormat(_) => "Unsupported character card format.",
            }
        }
}