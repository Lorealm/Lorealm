mod auth;
mod card;
mod permission;

pub use auth::*;
pub use card::*;
pub use permission::*;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use std::borrow::Cow;

use crate::services::character::CharacterServiceError;

/// Application-level error type exposed through API responses.
///
/// Each variant maps to a specific HTTP status code and a client-safe message.
/// Internal details are logged server-side but never leaked to the client.
pub enum AppError {
    Forbidden {
        code: &'static str,
        message: Cow<'static, str>,
    },
    Unauthorized {
        code: &'static str,
        message: Cow<'static, str>,
    },
    BadRequest {
        code: &'static str,
        message: Cow<'static, str>,
    },
    Conflict {
        code: &'static str,
        message: Cow<'static, str>,
    },
    Internal {
        internal: Cow<'static, str>,
    },
}

impl AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::BadRequest { .. } => "BAD_REQUEST",
            Self::Conflict { .. } => "CONFLICT",
            Self::Internal { .. } => "INTERNAL",
        }
    }

    /// The message included in the JSON response body.
    /// Internal errors always return a fixed generic message.
    fn client_message(&self) -> &str {
        match self {
            Self::Forbidden { message, code: _ }
            | Self::Unauthorized { message, code: _ }
            | Self::BadRequest { message, code: _ }
            | Self::Conflict { message, code: _ } => message,
            Self::Internal { .. } => "An internal error occurred",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        if let Self::Internal { ref internal } = self {
            // TODO: replace with your preferred logger (tracing, log, etc.)
            eprintln!("[ERROR] internal: {internal}");
        }

        let body = json!({
            "error_code": self.error_code(),
            "message": self.client_message(),
        });

        (self.status_code(), Json(body)).into_response()
    }
}

// ---------------------------------------------------------------------------
// Conversions
// ---------------------------------------------------------------------------

impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        // Log the detailed error for debugging; the client gets a safe message.
        tracing::error!("[ERROR] auth: {err}");

        match err {
            // Infrastructure failures → 500, details stay server-side
            AuthError::Encryption(_) | AuthError::Storage(_) | AuthError::TokenGenerationFailed => {
                AppError::Internal {
                    internal: Cow::Borrowed(err.client_message()),
                }
            }

            // Duplicate resource → 409
            AuthError::UsernameTaken(_) | AuthError::EmailTaken(_) => AppError::Conflict {
                message: Cow::Borrowed(err.client_message()),
                code: "USER_EXISTS",
            },

            // Malformed input → 400
            AuthError::MalformedCredentials => AppError::BadRequest {
                message: Cow::Borrowed(err.client_message()),
                code: "MALFORMED_CREDENTIALS",
            },

            // Everything else is an authentication/session issue → 401
            _ => AppError::Unauthorized {
                message: Cow::Borrowed(err.client_message()),
                code: "UNAUTHORIZED",
            },
        }
    }
}

impl From<PermissionError> for AppError {
    fn from(err: PermissionError) -> Self {
        tracing::error!("[ERROR] perms: {err}");

        match err {
            PermissionError::Missing(_) => AppError::Forbidden {
                code: "INVALID_PERMISSIONS",
                message: Cow::Borrowed(err.client_message()),
            },
            PermissionError::Storage(_) => AppError::Internal {
                internal: Cow::Borrowed(err.client_message()),
            },
        }
    }
}

impl From<CharacterServiceError> for AppError {
    fn from(err: CharacterServiceError) -> Self {
        tracing::error!("[ERROR] character service: {err}");

        match err {
            _ => AppError::Internal {
                internal: Cow::Borrowed(err.client_message()),
            },
        }
    }
}
