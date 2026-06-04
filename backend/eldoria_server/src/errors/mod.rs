mod auth;
mod card;
mod chat;
mod context;
mod conversation;
mod lorebook;
mod permission;
mod state;

pub use auth::*;
pub use card::*;
pub use chat::*;
pub use context::*;
pub use conversation::*;
pub use lorebook::*;
pub use permission::*;
pub use state::*;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use std::{
    borrow::Cow,
    fmt::Display,
};

use crate::services::character::CharacterServiceError;

#[derive(Debug)]
pub enum TargetType {
    Character,
    CharacterVersion,
    Greeting,
    Conversation,
    Message,
    ExampleMessage,
    ContextTemplate,
}

impl Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Character => write!(f, "character"),
            TargetType::CharacterVersion => write!(f, "character_version"),
            TargetType::Greeting => write!(f, "greeting"),
            TargetType::Conversation => write!(f, "conversation"),
            TargetType::Message => write!(f, "message"),
            TargetType::ExampleMessage => write!(f, "example_message"),
            TargetType::ContextTemplate => write!(f, "context_template"),
        }
    }
}

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

    fn error_code(&self) -> String {
        match self {
            Self::Forbidden { code, message: _ } => format!("FORBIDDEN: {code}"),
            Self::Unauthorized { code, message: _ } => format!("UNAUTHORIZED: {code}"),
            Self::BadRequest { code, message: _ } => format!("BAD_REQUEST: {code}"),
            Self::Conflict { code, message: _ } => format!("CONFLICT: {code}"),
            Self::Internal { .. } => "INTERNAL".to_string(),
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

impl From<ContextError> for AppError {
    fn from(err: ContextError) -> Self {
        tracing::error!("[ERROR] context service: {err}");

        match err {
            ContextError::NotFound(_, _) => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            ContextError::Lorebook(e) => match e {
                LorebookError::NotOwned(_, _) => AppError::BadRequest {
                    code: "NOT_FOUND",
                    message: Cow::Owned(e.client_message()),
                },
                LorebookError::ContextWeaver(_) => AppError::BadRequest {
                    code: "CONTEXT_WEAVER",
                    message: Cow::Owned(e.client_message()),
                },
                LorebookError::Yaml(_) => AppError::BadRequest {
                    code: "SERIALIZATION",
                    message: Cow::Owned(e.client_message()),
                },
            },
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<ChatError> for AppError {
    fn from(err: ChatError) -> Self {
        tracing::error!("[ERROR] chat service: {err}");

        match err {
            ChatError::NotFound(_, _) | ChatError::NotOwned(_, _) => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            ChatError::Character(character_service_error) => character_service_error.into(),
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<StateError> for AppError {
    fn from(err: StateError) -> Self {
        match err {
            StateError::NoActiveTemplate | StateError::NotFound(_, _ ) => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}
