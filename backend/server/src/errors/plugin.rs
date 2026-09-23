use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::StorageError;
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error};

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin {0} is not installed")]
    NotFound(String),
    #[error("plugin {0} failed to load: {1}")]
    Invalid(String, lorealm_plugins::PluginError),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for PluginError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "plugin.not_found",
            Self::Invalid(_, _) => "plugin.invalid",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Invalid(_, _) => StatusCode::BAD_REQUEST,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_) => "That plugin is not installed.".into(),
            Self::Invalid(_, _) => "That plugin could not be loaded.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(_) | Self::Invalid(_, _) => {
                Some(ErrorDetails::resource(TargetType::Plugin))
            }
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(PluginError);
