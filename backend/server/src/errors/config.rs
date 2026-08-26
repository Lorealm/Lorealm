use std::borrow::Cow;

use axum::http::StatusCode;

use crate::errors::{DomainError, impl_app_error};

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown key")]
    UnknownKey,
    #[error("invalid: {0}")]
    Invalid(serde_json::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
}

impl DomainError for ConfigError {
    fn code(&self) -> &'static str {
        match self {
            Self::UnknownKey => "config.unknown_key",
            Self::Invalid(_) | Self::Io(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::UnknownKey => StatusCode::BAD_REQUEST,
            Self::Invalid(_) | Self::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::UnknownKey => "Unknown configuration key.".into(),
            Self::Invalid(_) | Self::Io(_) => "An internal error occurred".into(),
        }
    }
}

impl_app_error!(ConfigError);
