use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{
    CharacterError, ConnectionError, ContextError, ConversationError, DomainError, ErrorDetails,
    PersonaError, StateError, TargetType, TokenizerError, UserToolError, impl_app_error, sentence,
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
    #[error("tool error: {0}")]
    UserTool(#[from] UserToolError),
    #[error("tokenizer error: {0}")]
    Tokenizer(#[from] TokenizerError),
    #[error("conversation {0} is a realm playthrough")]
    RealmConversation(Snowflake),
}

impl DomainError for ChatError {
    fn code(&self) -> &'static str {
        match self {
            Self::Character(err) => err.code(),
            Self::Conversation(err) => err.code(),
            Self::Context(err) => err.code(),
            Self::State(err) => err.code(),
            Self::Persona(err) => err.code(),
            Self::ConnectionProfile(err) => err.code(),
            Self::UserTool(err) => err.code(),
            Self::Tokenizer(err) => err.code(),
            Self::NotFound(_, _) | Self::NotOwned(_, _) => "chat.not_found",
            // Both are failures of the upstream LLM call.
            Self::Provider(_) | Self::Connection(_) => "chat.provider_failed",
            Self::RealmConversation(_) => "chat.realm_conversation",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Character(err) => err.status(),
            Self::Conversation(err) => err.status(),
            Self::Context(err) => err.status(),
            Self::State(err) => err.status(),
            Self::Persona(err) => err.status(),
            Self::ConnectionProfile(err) => err.status(),
            Self::UserTool(err) => err.status(),
            Self::Tokenizer(err) => err.status(),
            Self::NotFound(_, _) | Self::NotOwned(_, _) => StatusCode::NOT_FOUND,
            Self::Provider(_) | Self::Connection(_) => StatusCode::BAD_GATEWAY,
            Self::RealmConversation(_) => StatusCode::CONFLICT,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Character(err) => err.message(),
            Self::Conversation(err) => err.message(),
            Self::Context(err) => err.message(),
            Self::State(err) => err.message(),
            Self::Persona(err) => err.message(),
            Self::ConnectionProfile(err) => err.message(),
            Self::UserTool(err) => err.message(),
            Self::Tokenizer(err) => err.message(),
            Self::NotFound(target, _) | Self::NotOwned(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::Provider(err) => sentence(format!("The model provider failed: {err}")),
            Self::Connection(reason) => {
                sentence(format!("Could not reach the model provider: {reason}"))
            }
            Self::RealmConversation(_) => {
                "Realm conversations advance through their turn endpoints.".into()
            }
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::Character(err) => err.details(),
            Self::Conversation(err) => err.details(),
            Self::Context(err) => err.details(),
            Self::State(err) => err.details(),
            Self::Persona(err) => err.details(),
            Self::ConnectionProfile(err) => err.details(),
            Self::UserTool(err) => err.details(),
            Self::Tokenizer(err) => err.details(),
            Self::NotFound(target, id) | Self::NotOwned(target, id) => {
                Some(ErrorDetails::target(*target, *id))
            }
            Self::RealmConversation(id) => {
                Some(ErrorDetails::target(TargetType::Conversation, *id))
            }
            Self::Provider(_) | Self::Connection(_) => {
                Some(ErrorDetails::resource(TargetType::ConnectionProfile))
            }
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(ChatError);
