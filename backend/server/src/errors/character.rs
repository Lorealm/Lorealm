use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{error::StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::{
    CardParseError, DomainError, ErrorDetails, TargetType, impl_app_error, sentence,
};

#[derive(Error, Debug)]
pub enum CharacterError {
    #[error("failed to parse character card: {0}")]
    Parse(#[from] CardParseError),

    #[error("storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("failed to deserialize card JSON: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("unsupported card format: '{0}' (expected chara_card_v2)")]
    UnsupportedFormat(String),
    #[error("{0} {1} does not exist")]
    NotFound(TargetType, Snowflake),
    #[error("character not owned: {0}")]
    NotOwned(Snowflake),
    #[error("permission denied")]
    PermissionDenied,
    #[error("unsupported media type")]
    UnsupportedMediaType,
}

impl DomainError for CharacterError {
    fn code(&self) -> &'static str {
        match self {
            // Both describe the uploaded card, not our internals.
            Self::Parse(_) | Self::Deserialization(_) => "character.invalid_card",
            Self::UnsupportedFormat(_) => "character.unsupported_format",
            // NotOwned and PermissionDenied share the not-found code so the API
            // never reveals that a character exists under another account.
            Self::NotFound(_, _) | Self::NotOwned(_) | Self::PermissionDenied => {
                "character.not_found"
            }
            Self::UnsupportedMediaType => "character.unsupported_media_type",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Parse(_) | Self::Deserialization(_) | Self::UnsupportedFormat(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::NotFound(_, _) | Self::NotOwned(_) | Self::PermissionDenied => {
                StatusCode::NOT_FOUND
            }
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Parse(err) => sentence(format!("Character card could not be read: {err}")),
            Self::Deserialization(_) => "Character card contains invalid JSON.".into(),
            Self::UnsupportedFormat(format) => Cow::Owned(format!(
                "Unsupported card format '{format}' — expected chara_card_v2."
            )),
            Self::NotFound(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::NotOwned(_) | Self::PermissionDenied => "That character no longer exists.".into(),
            Self::UnsupportedMediaType => "That image format isn't supported.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(target, id) => Some(ErrorDetails::target(*target, *id)),
            Self::NotOwned(id) => Some(ErrorDetails::target(TargetType::Character, *id)),
            Self::Parse(_)
            | Self::Deserialization(_)
            | Self::UnsupportedFormat(_)
            | Self::PermissionDenied
            | Self::UnsupportedMediaType => Some(ErrorDetails::resource(TargetType::Character)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(CharacterError);
