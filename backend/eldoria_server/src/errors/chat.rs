use std::fmt::Display;

use eldoria_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::{errors::ConversationError, services::character::CharacterServiceError};

#[derive(Debug)]
pub enum TargetType {
    Character,
    Greeting,
    Conversation,
    Message
}

impl Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Character => write!(f, "character"),
            TargetType::Greeting => write!(f, "greeting"),
            TargetType::Conversation => write!(f, "conversation"),
            TargetType::Message => write!(f, "message"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("character error: {0}")]
    Character(#[from] CharacterServiceError),
    #[error("greeting not found: {0}")]
    NotFound(TargetType, Snowflake),
    #[error("conversation not owned: {0}")]
    NotOwned(TargetType, Snowflake),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("conversation error: {0}")]
    Conversation(#[from] ConversationError),
}

impl ChatError {
    pub fn client_message(&self) -> String {
        match self {
            ChatError::Character(character_service_error) => character_service_error.client_message().to_string(),
            ChatError::NotFound(target_type, snowflake) => format!("{target_type} not found: {snowflake}"),
            ChatError::NotOwned(target_type, snowflake) => format!("{target_type} not owned: {snowflake}"),
            ChatError::Storage(storage_error) => storage_error.to_string(),
            ChatError::Conversation(conversation_error) => conversation_error.to_string(),
        }
    }
}