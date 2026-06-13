use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{
    CharacterError, ConnectionError, ContextError, ConversationError, PersonaError, StateError,
    TargetType,
};

#[derive(Debug, Error)]
pub enum ChatError {
    #[error("character error: {0}")]
    Character(#[from] CharacterError),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("conversation not owned: {0}")]
    NotOwned(TargetType, Snowflake),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("conversation error: {0}")]
    Conversation(#[from] ConversationError),
    #[error("context error: {0}")]
    Context(#[from] ContextError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("persona error: {0}")]
    Persona(#[from] PersonaError),
    #[error("connection profile error: {0}")]
    ConnectionProfile(#[from] ConnectionError),
    #[error("provider error: {0}")]
    Provider(#[from] lorealm_providers::Error),
    #[error("connection error: {0}")]
    Connection(String),
}

impl ChatError {
    pub fn client_message(&self) -> String {
        match self {
            ChatError::Character(character_service_error) => {
                character_service_error.client_message().to_string()
            }
            ChatError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            ChatError::NotOwned(target_type, snowflake) => {
                format!("{target_type} not owned: {snowflake}")
            }
            ChatError::Storage(storage_error) => storage_error.to_string(),
            ChatError::Conversation(conversation_error) => conversation_error.to_string(),
            ChatError::Context(context_error) => context_error.client_message(),
            ChatError::State(state_error) => state_error.client_message(),
            ChatError::Persona(persona_error) => persona_error.client_message(),
            ChatError::ConnectionProfile(connection_error) => connection_error.client_message(),
            ChatError::Provider(error) => error.to_string(),
            ChatError::Connection(msg) => msg.to_owned(),
        }
    }
}
