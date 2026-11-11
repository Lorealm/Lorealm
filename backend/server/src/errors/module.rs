use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, Error)]
pub enum ModuleError {
    #[error("module {0} not found")]
    NotFound(Snowflake),
    #[error("module name '{0}' is taken")]
    NameTaken(String),
    #[error("invalid module: {0}")]
    Invalid(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for ModuleError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "module.not_found",
            Self::NameTaken(_) => "module.name_taken",
            Self::Invalid(_) => "module.invalid",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::NameTaken(_) => StatusCode::CONFLICT,
            Self::Invalid(_) => StatusCode::BAD_REQUEST,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_) => "That module no longer exists.".into(),
            Self::NameTaken(name) => sentence(format!("The name '{name}' is already in use")),
            Self::Invalid(reason) => sentence(format!("Invalid module: {reason}")),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(id) => Some(ErrorDetails::target(TargetType::Module, *id)),
            Self::NameTaken(_) | Self::Invalid(_) => {
                Some(ErrorDetails::resource(TargetType::Module))
            }
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(ModuleError);
