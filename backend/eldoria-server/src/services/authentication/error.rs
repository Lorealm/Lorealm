use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::models::UserDeviceError;

#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("Missing session token")]
    MissingToken,
    #[error("Invalid session token")]
    InvalidToken,
    #[error("Session expired")]
    SessionExpired,
    #[error("Session revoked")]
    SessionRevoked,
    #[error("Device mismatch")]
    DeviceMismatch,
    #[error("Unexpected cryptographic error: {0}")]
    Cryptography(#[from] encryption_service::EncryptionError),
    #[error("Persistence error: {0}")]
    Persistence(#[from] storage_system::StorageError),
    #[error("User device error: {0}")]
    UserDevice(#[from] UserDeviceError),
    #[error("Token generation failed")]
    TokenGenerationFailed,
    #[error("Missing CSRF Token")]
    MissingCsrfToken,
    #[error("Invalid CSRF Token")]
    InvalidCsrfToken,
}

impl IntoResponse for SessionError {
    fn into_response(self) -> Response {
        let status = match self {
            SessionError::MissingToken | SessionError::InvalidToken => StatusCode::UNAUTHORIZED,
            SessionError::InvalidCsrfToken | SessionError::MissingCsrfToken => StatusCode::FORBIDDEN,
            SessionError::SessionExpired | SessionError::SessionRevoked => StatusCode::UNAUTHORIZED,
            SessionError::DeviceMismatch => StatusCode::FORBIDDEN,
            SessionError::Cryptography(_) | SessionError::TokenGenerationFailed => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            SessionError::Persistence(_) => StatusCode::INTERNAL_SERVER_ERROR,
            SessionError::UserDevice(_) => StatusCode::BAD_REQUEST,
        };

        (status, self.to_string()).into_response()
    }
}
