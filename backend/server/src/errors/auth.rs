use lorealm_encryption::EncryptionError;
use lorealm_storage::StorageError;
use thiserror::Error;

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
}

impl AuthError {
    /// The message safe to show to API clients.
    /// Security-sensitive variants deliberately share vague messages
    /// to avoid leaking whether a user/email exists.
    pub fn client_message(&self) -> &'static str {
        match self {
            Self::InvalidSession | Self::Revoked | Self::Expired | Self::TokenGenerationFailed => {
                "Invalid or expired session"
            }
            Self::InvalidCsrfToken | Self::MissingCsrfToken => "Invalid or missing CSRF token",
            Self::MissingToken => "Missing authentication token",
            Self::MalformedCredentials | Self::UserNotFound | Self::IncorrectPassword => {
                "Invalid credentials"
            }
            Self::UsernameTaken(_) => "Username is already taken",
            Self::EmailTaken(_) => "Email is already taken",
            // Internal details are never exposed to clients
            Self::Encryption(_) | Self::Storage(_) => "An internal error occurred",
        }
    }
}
