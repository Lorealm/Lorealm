use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error};

#[derive(Debug, Error)]
pub enum PersonaError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("persona not owned")]
    NotOwned,
    #[error("unsupported media type")]
    UnsupportedMediaType,
    #[error("cannot delete the last persona")]
    LastPersona,
}

impl DomainError for PersonaError {
    fn code(&self) -> &'static str {
        match self {
            // NotOwned shares the not-found code so the API never reveals that
            // a persona exists under another account.
            Self::NotFound(_, _) | Self::NotOwned => "persona.not_found",
            Self::UnsupportedMediaType => "persona.unsupported_media_type",
            Self::LastPersona => "persona.last_persona",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) | Self::NotOwned => StatusCode::NOT_FOUND,
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::LastPersona => StatusCode::CONFLICT,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::NotOwned => "That persona no longer exists.".into(),
            Self::UnsupportedMediaType => "That image format isn't supported.".into(),
            Self::LastPersona => {
                "You need at least one persona. Create another one before deleting this one.".into()
            }
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) => Some(ErrorDetails::target(*target, *id)),
            Self::NotOwned => Some(ErrorDetails::resource(TargetType::Persona)),
            Self::UnsupportedMediaType | Self::LastPersona | Self::Storage(_) => None,
        }
    }
}

impl_app_error!(PersonaError);
