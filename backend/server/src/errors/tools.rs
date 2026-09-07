use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, thiserror::Error)]
pub enum UserToolError {
    #[error("user tool {1} not found")]
    NotFound(TargetType, Snowflake),
    #[error("user tool not owned by caller")]
    NotOwned,
    #[error("invalid tool name: {0}")]
    InvalidName(String),
    #[error("tool body failed to compile: {0}")]
    InvalidBody(String),
    #[error("parameter '{name}': {reason}")]
    InvalidParam { name: String, reason: String },
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for UserToolError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => "tool.not_found",
            Self::InvalidName(_) => "tool.invalid_name",
            Self::InvalidBody(_) => "tool.invalid_body",
            Self::InvalidParam { .. } => "tool.invalid_param",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => StatusCode::NOT_FOUND,
            Self::InvalidName(_) | Self::InvalidBody(_) | Self::InvalidParam { .. } => {
                StatusCode::BAD_REQUEST
            }
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => "That tool no longer exists.".into(),
            // Both carry validation output describing the user's own tool.
            Self::InvalidName(reason) => sentence(format!("Invalid tool name: {reason}")),
            Self::InvalidBody(reason) => sentence(format!("Tool failed to compile: {reason}")),
            Self::InvalidParam { name, reason } => {
                sentence(format!("Parameter '{name}': {reason}"))
            }
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) => Some(ErrorDetails::target(*target, *id)),
            Self::NotOwned
            | Self::InvalidName(_)
            | Self::InvalidBody(_)
            | Self::InvalidParam { .. } => Some(ErrorDetails::resource(TargetType::UserTool)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(UserToolError);
