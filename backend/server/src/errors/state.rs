use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error};

#[derive(Debug, Error)]
pub enum StateError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("no active context template")]
    NoActiveTemplate,
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("no active persona")]
    NoActivePersona,
    #[error("no active connection profile")]
    NoActiveConnectionProfile,
    #[error("no active tokenizer")]
    NoActiveTokenizer,
}

impl DomainError for StateError {
    fn code(&self) -> &'static str {
        match self {
            Self::NoActiveTemplate => "state.no_active_template",
            Self::NotFound(_, _) => "state.not_found",
            Self::NoActivePersona => "state.no_active_persona",
            Self::NoActiveConnectionProfile => "state.no_active_connection_profile",
            Self::NoActiveTokenizer => "state.no_active_tokenizer",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) => StatusCode::NOT_FOUND,
            // The request is well-formed; the workspace just isn't configured
            // for it yet.
            Self::NoActiveTemplate
            | Self::NoActivePersona
            | Self::NoActiveConnectionProfile
            | Self::NoActiveTokenizer => StatusCode::CONFLICT,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NoActiveTemplate => "No context template is selected.".into(),
            Self::NotFound(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::NoActivePersona => "No persona is selected.".into(),
            Self::NoActiveConnectionProfile => "No connection profile is selected.".into(),
            Self::NoActiveTokenizer => "No tokenizer is selected.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) => Some(ErrorDetails::target(*target, *id)),
            Self::NoActiveTemplate => Some(ErrorDetails::resource(TargetType::ContextTemplate)),
            Self::NoActivePersona => Some(ErrorDetails::resource(TargetType::Persona)),
            Self::NoActiveConnectionProfile => {
                Some(ErrorDetails::resource(TargetType::ConnectionProfile))
            }
            Self::NoActiveTokenizer => Some(ErrorDetails::resource(TargetType::Tokenizer)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(StateError);
