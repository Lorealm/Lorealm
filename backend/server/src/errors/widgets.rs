use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, Error)]
pub enum UserWidgetError {
    #[error("widget {1} not found")]
    NotFound(TargetType, Snowflake),
    #[error("widget not owned by caller")]
    NotOwned,
    #[error("invalid widget: {0}")]
    Invalid(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for UserWidgetError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => "widget.not_found",
            Self::Invalid(_) => "widget.invalid",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => StatusCode::NOT_FOUND,
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => "That widget no longer exists.".into(),
            Self::Invalid(reason) => sentence(format!("Invalid widget: {reason}")),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) => Some(ErrorDetails::target(*target, *id)),
            Self::NotOwned | Self::Invalid(_) => {
                Some(ErrorDetails::resource(TargetType::UserWidget))
            }
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(UserWidgetError);
