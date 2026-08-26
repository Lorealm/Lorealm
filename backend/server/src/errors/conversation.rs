use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error};

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("conversation not found: {0}")]
    ConversationNotFound(Snowflake),
    #[error("message {0} not found in this conversation")]
    MessageNotFound(Snowflake),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for ConversationError {
    fn code(&self) -> &'static str {
        match self {
            Self::ConversationNotFound(_) => "conversation.not_found",
            Self::MessageNotFound(_) => "conversation.message_not_found",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::ConversationNotFound(_) | Self::MessageNotFound(_) => StatusCode::NOT_FOUND,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::ConversationNotFound(_) => "That conversation no longer exists.".into(),
            Self::MessageNotFound(_) => "That message is no longer in this conversation.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::ConversationNotFound(id) => {
                Some(ErrorDetails::target(TargetType::Conversation, *id))
            }
            Self::MessageNotFound(id) => Some(ErrorDetails::target(TargetType::Message, *id)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(ConversationError);
