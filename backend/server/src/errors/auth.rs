use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_encryption::EncryptionError;
use lorealm_storage::StorageError;
use thiserror::Error;

use crate::errors::{DomainError, impl_app_error};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid session")]
    InvalidSession,
    #[error("Session revoked")]
    Revoked,
    #[error("Session expired")]
    Expired,
    #[error("Invalid CSRF token")]
    InvalidCsrfToken,
    #[error("Token generation failed")]
    TokenGenerationFailed,
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Missing auth token")]
    MissingToken,
    #[error("Missing CSRF token")]
    MissingCsrfToken,
    #[error("Malformed credentials")]
    MalformedCredentials,
    #[error("User not found")]
    UserNotFound,
    #[error("Incorrect password")]
    IncorrectPassword,
    #[error("Username taken: {0}")]
    UsernameTaken(String),
    #[error("Email taken: {0}")]
    EmailTaken(String),
    #[error("Display name too long")]
    DisplayNameTooLong,
    #[error("Unsupported avatar media type")]
    UnsupportedAvatarType,
    #[error("Current password is incorrect")]
    WrongCurrentPassword,
    #[error("Password must be {min}-{max} characters")]
    PasswordLength { min: usize, max: usize },
}

impl DomainError for AuthError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidSession | Self::Revoked | Self::Expired => "auth.session_invalid",
            Self::InvalidCsrfToken | Self::MissingCsrfToken => "auth.csrf_invalid",
            Self::MissingToken => "auth.missing_token",
            Self::MalformedCredentials | Self::UserNotFound | Self::IncorrectPassword => {
                "auth.invalid_credentials"
            }
            Self::UsernameTaken(_) => "auth.username_taken",
            Self::EmailTaken(_) => "auth.email_taken",
            Self::WrongCurrentPassword => "auth.wrong_current_password",
            Self::PasswordLength { .. } => "auth.password_length",
            Self::DisplayNameTooLong => "user.display_name_too_long",
            Self::UnsupportedAvatarType => "user.unsupported_avatar_type",
            Self::TokenGenerationFailed | Self::Encryption(_) | Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::UsernameTaken(_) | Self::EmailTaken(_) => StatusCode::CONFLICT,
            Self::DisplayNameTooLong | Self::PasswordLength { .. } => StatusCode::BAD_REQUEST,
            Self::WrongCurrentPassword => StatusCode::FORBIDDEN,
            Self::UnsupportedAvatarType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::TokenGenerationFailed | Self::Encryption(_) | Self::Storage(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            _ => StatusCode::UNAUTHORIZED,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::InvalidSession | Self::Revoked | Self::Expired => {
                "Your session has expired. Please sign in again.".into()
            }
            Self::InvalidCsrfToken | Self::MissingCsrfToken => {
                "Invalid or missing CSRF token.".into()
            }
            Self::MissingToken => "You are not signed in.".into(),
            Self::MalformedCredentials | Self::UserNotFound | Self::IncorrectPassword => {
                "Invalid credentials.".into()
            }
            Self::UsernameTaken(_) => "That username is already taken.".into(),
            Self::EmailTaken(_) => "That email is already registered.".into(),
            Self::DisplayNameTooLong => "That display name is too long.".into(),
            Self::WrongCurrentPassword => "Your current password is incorrect.".into(),
            Self::PasswordLength { min, max } => {
                format!("Passwords must be between {min} and {max} characters.").into()
            }
            Self::UnsupportedAvatarType => "Avatars must be a PNG, JPEG, WebP or GIF image.".into(),
            Self::TokenGenerationFailed | Self::Encryption(_) | Self::Storage(_) => {
                "An internal error occurred".into()
            }
        }
    }
}

impl_app_error!(AuthError);
