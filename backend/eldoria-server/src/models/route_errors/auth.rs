use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;

use crate::{models::users::UserError, services::authentication::error::SessionError};

#[derive(thiserror::Error, Debug)]
pub enum AuthError {
    #[error("User error: {0}")]
    User(#[from] UserError),
    #[error("Session error: {0}")]
    Session(#[from] SessionError),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match &self {
            AuthError::User(user_err) => match user_err {
                UserError::RegistrationError(_) => (
                    StatusCode::BAD_REQUEST,
                    "registration_error",
                    "Registration failed",
                ),
                UserError::PermissionError { .. } => (
                    StatusCode::FORBIDDEN,
                    "permission_denied",
                    "Permission denied",
                ),
                UserError::InvalidUser(_) => (
                    StatusCode::UNAUTHORIZED,
                    "invalid_user",
                    "Invalid credentials",
                ),
                UserError::IncorrectPassword => (
                    StatusCode::UNAUTHORIZED,
                    "incorrect_password",
                    "Invalid credentials",
                ),
                UserError::Encryption(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "encryption_error",
                    "An internal error occurred",
                ),
            },
            AuthError::Session(session_err) => match session_err {
                SessionError::MissingToken => (
                    StatusCode::UNAUTHORIZED,
                    "missing_token",
                    "Authentication required",
                ),
                SessionError::InvalidToken => (
                    StatusCode::UNAUTHORIZED,
                    "invalid_token",
                    "Invalid authentication token",
                ),
                SessionError::MissingCsrfToken => (
                    StatusCode::FORBIDDEN,
                    "missing_csrf_token",
                    "Authentication required"
                ),
                SessionError::InvalidCsrfToken => (
                    StatusCode::FORBIDDEN,
                    "invalid_csrf_token",
                    "Invalid CSRF token",
                ),
                SessionError::SessionExpired => (
                    StatusCode::UNAUTHORIZED,
                    "session_expired",
                    "Session expired",
                ),
                SessionError::SessionRevoked => (
                    StatusCode::UNAUTHORIZED,
                    "session_revoked",
                    "Session revoked",
                ),
                SessionError::DeviceMismatch => (
                    StatusCode::FORBIDDEN,
                    "device_mismatch",
                    "Access denied",
                ),
                SessionError::Cryptography(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "cryptography_error",
                    "An internal error occurred",
                ),
                SessionError::Persistence(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "persistence_error",
                    "An internal error occurred",
                ),
                SessionError::UserDevice(_) => (
                    StatusCode::BAD_REQUEST,
                    "user_device_error",
                    "Invalid request",
                ),
                SessionError::TokenGenerationFailed => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "token_generation_failed",
                    "An internal error occurred",
                ),
            },
        };

        tracing::error!("Auth error: {:?}", self); // Log the detailed error

        let body = Json(json!({
            "error": message,
            "error_type": error_type,
        }));

        (status, body).into_response()
    }
}