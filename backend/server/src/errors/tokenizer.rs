use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    #[error("tokenizer {0} not found")]
    NotFound(Snowflake),
    #[error("not permitted to modify tokenizer {0}")]
    Forbidden(Snowflake),
    #[error("invalid tokenizer data: {0}")]
    InvalidData(String),
    #[error("invalid tokenizer source: {0}")]
    InvalidSource(String),
    #[error("failed to fetch tokenizer: {0}")]
    Fetch(String),
    #[error("failed to tokenize: {0}")]
    Tokenize(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for TokenizerError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "tokenizer.not_found",
            Self::Forbidden(_) => "tokenizer.forbidden",
            Self::InvalidData(_) => "tokenizer.invalid_data",
            Self::InvalidSource(_) => "tokenizer.invalid_source",
            Self::Fetch(_) => "tokenizer.fetch_failed",
            Self::Tokenize(_) => "tokenizer.tokenize_failed",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Forbidden(_) => StatusCode::FORBIDDEN,
            Self::InvalidData(_) | Self::InvalidSource(_) | Self::Tokenize(_) => {
                StatusCode::BAD_REQUEST
            }
            // The upstream download failed, not us.
            Self::Fetch(_) => StatusCode::BAD_GATEWAY,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_) => "That tokenizer no longer exists.".into(),
            Self::Forbidden(_) => "This tokenizer can't be modified.".into(),
            Self::InvalidData(_) => "The imported tokenizer was invalid.".into(),
            Self::InvalidSource(source) => {
                Cow::Owned(format!("'{source}' is not a valid tokenizer source."))
            }
            Self::Fetch(reason) => sentence(format!("Failed to download tokenizer: {reason}")),
            Self::Tokenize(reason) => sentence(format!("Failed to tokenize: {reason}")),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(id) | Self::Forbidden(id) => {
                Some(ErrorDetails::target(TargetType::Tokenizer, *id))
            }
            Self::InvalidData(_) | Self::InvalidSource(_) | Self::Fetch(_) | Self::Tokenize(_) => {
                Some(ErrorDetails::resource(TargetType::Tokenizer))
            }
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(TokenizerError);
