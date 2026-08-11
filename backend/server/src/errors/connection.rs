use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_encryption::EncryptionError;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("{0:?} {1} not found")]
    NotFound(TargetType, Snowflake),

    #[error("{0:?} {1} is not owned by this user")]
    NotOwned(TargetType, Snowflake),

    #[error("validation failed: {0}")]
    Validation(String),

    /// Stored encrypted payload failed structural reconstruction (wrong
    /// nonce/tag size, mismatched context). Indicates corruption or
    /// tampering, not user error.
    #[error("secret {0} payload is corrupted")]
    Corrupted(Snowflake),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Encryption(#[from] EncryptionError),
}

impl DomainError for ConnectionError {
    fn code(&self) -> &'static str {
        match self {
            // NotOwned shares the not-found code so the API never reveals that
            // a profile exists under another account.
            Self::NotFound(_, _) | Self::NotOwned(_, _) => "connection.not_found",
            Self::Validation(_) => "connection.invalid_profile",
            Self::Corrupted(_) => "connection.secret_corrupted",
            Self::Storage(_) | Self::Encryption(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_, _) | Self::NotOwned(_, _) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::BAD_REQUEST,
            // Recoverable by re-entering the key, so it stays a client error.
            Self::Corrupted(_) => StatusCode::CONFLICT,
            Self::Storage(_) | Self::Encryption(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(target, _) | Self::NotOwned(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::Validation(reason) => sentence(reason.clone()),
            Self::Corrupted(_) => {
                "This API key could not be decrypted and needs to be entered again.".into()
            }
            Self::Storage(_) | Self::Encryption(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) | Self::NotOwned(target, id) => {
                Some(ErrorDetails::target(*target, *id))
            }
            Self::Corrupted(id) => Some(ErrorDetails::target(TargetType::Secret, *id)),
            Self::Validation(_) => Some(ErrorDetails::resource(TargetType::ConnectionProfile)),
            Self::Storage(_) | Self::Encryption(_) => None,
        }
    }
}

impl_app_error!(ConnectionError);
