mod auth;
mod card;
mod character;
mod chat;
mod config;
mod connection;
mod context;
mod conversation;
mod lorebook;
mod permission;
mod persona;
mod realm;
mod state;
mod tokenizer;
mod tools;

pub use auth::*;
pub use card::*;
pub use character::*;
pub use chat::*;
pub use config::*;
pub use connection::*;
pub use context::*;
pub use conversation::*;
pub use lorebook::*;
pub use permission::*;
pub use persona::*;
pub use realm::*;
pub use state::*;
pub use tokenizer::*;
pub use tools::*;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use std::{borrow::Cow, fmt::Display};
use thiserror::Error;

#[derive(Debug)]
pub enum TargetType {
    Character,
    CharacterVersion,
    Greeting,
    Conversation,
    Message,
    ExampleMessage,
    ContextTemplate,
    Persona,
    ConnectionProfile,
    Secret,
    UserTool,
    Realm,
    RealmAgent,
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
            TargetType::Persona => write!(f, "persona"),
            TargetType::ConnectionProfile => write!(f, "connection_profile"),
            TargetType::Secret => write!(f, "secret"),
            TargetType::UserTool => write!(f, "user_tool"),
            TargetType::Realm => write!(f, "realm"),
            TargetType::RealmAgent => write!(f, "realm_agent"),
        }
    }
}

/// Application-level error type exposed through API responses.
///
/// Each variant maps to a specific HTTP status code and a client-safe message.
/// Internal details are logged server-side but never leaked to the client.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("FORBIDDEN ({code}): {message}")]
    Forbidden {
        code: &'static str,
        message: Cow<'static, str>,
    },
    #[error("UNAUTHORIZED ({code}): {message}")]
    Unauthorized {
        code: &'static str,
        message: Cow<'static, str>,
    },
    #[error("BAD_REQUEST ({code}): {message}")]
    BadRequest {
        code: &'static str,
        message: Cow<'static, str>,
    },
    #[error("CONFLICT ({code}): {message}")]
    Conflict {
        code: &'static str,
        message: Cow<'static, str>,
    },
    #[error("INTERNAL: {internal}")]
    Internal { internal: Cow<'static, str> },
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
            tracing::error!("[ERROR] internal: {internal}");
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

impl From<CharacterError> for AppError {
    fn from(err: CharacterError) -> Self {
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
            ChatError::State(state_error) => state_error.into(),
            ChatError::Connection(err) => AppError::BadRequest {
                code: "CONNECTION",
                message: Cow::Owned(format!("connection error: {err}")),
            },
            ChatError::RealmConversation(_) => AppError::BadRequest {
                code: "REALM_CONVERSATION",
                message: Cow::Owned(err.client_message()),
            },
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<StateError> for AppError {
    fn from(err: StateError) -> Self {
        match err {
            StateError::NoActiveTemplate
            | StateError::NoActiveConnectionProfile
            | StateError::NoActivePersona
            | StateError::NotFound(_, _) => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<PersonaError> for AppError {
    fn from(err: PersonaError) -> Self {
        match err {
            _ => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<ConnectionError> for AppError {
    fn from(err: ConnectionError) -> Self {
        match err {
            ConnectionError::NotFound(_, _) | ConnectionError::NotOwned(_, _) => {
                AppError::BadRequest {
                    code: "NOT_FOUND",
                    message: Cow::Owned(err.client_message()),
                }
            }
            ConnectionError::Validation(_) | ConnectionError::Corrupted(_) => {
                AppError::BadRequest {
                    code: "INVALID_DATA",
                    message: Cow::Owned(err.client_message()),
                }
            }
            ConnectionError::Encryption(_) | ConnectionError::Storage(_) => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<UserToolError> for AppError {
    fn from(err: UserToolError) -> Self {
        match err {
            UserToolError::NotFound(_, _) | UserToolError::NotOwned => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            UserToolError::InvalidName(_) | UserToolError::InvalidBody(_) => AppError::BadRequest {
                code: "INVALID_DATA",
                message: Cow::Owned(err.client_message()),
            },
            UserToolError::Storage(_) => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<TokenizerError> for AppError {
    fn from(err: TokenizerError) -> Self {
        match err {
            TokenizerError::NotFound(_) | TokenizerError::Forbidden(_) => AppError::BadRequest {
                code: "NOT_FOUND",
                message: Cow::Owned(err.client_message()),
            },
            TokenizerError::InvalidData(_)
            | TokenizerError::InvalidSource(_)
            | TokenizerError::Fetch(_)
            | TokenizerError::Tokenize(_) => AppError::BadRequest {
                code: "INVALID_DATA",
                message: Cow::Owned(err.client_message()),
            },
            TokenizerError::Storage(_) => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        match err {
            ConfigError::UnknownKey => AppError::BadRequest {
                code: "INVALID_DATA",
                message: Cow::Owned(err.client_message()),
            },
            ConfigError::Invalid(_) | ConfigError::Io(_) => AppError::Internal {
                internal: Cow::Owned(err.client_message()),
            },
        }
    }
}

impl From<RealmError> for AppError {
    fn from(err: RealmError) -> Self {
        match err {
            RealmError::NotYourTurn(_) => AppError::Conflict {
                code: "CONFLICT",
                message: Cow::Owned(err.client_message()),
            },
            RealmError::NotPlayable(_)
            | RealmError::NotRealmConversation(_)
            | RealmError::NoAgents(_)
            | RealmError::Create(_) => AppError::BadRequest {
                code: "INVALID_REALM",
                message: Cow::Owned(err.client_message()),
            },
            RealmError::MalformedAgent(_, _) | RealmError::InvalidAgent(_) => {
                AppError::BadRequest {
                    code: "INVALID_REALM_AGENT",
                    message: Cow::Owned(err.client_message()),
                }
            }
            RealmError::NotFound(_, _) | RealmError::NotOwned(_, _) => AppError::BadRequest {
                code: "INVALID_RESOURCE",
                message: Cow::Owned(err.client_message()),
            },
            RealmError::InvalidTurn(_) | RealmError::ToolNotBound(_) => AppError::BadRequest {
                code: "INVALID_TURN",
                message: Cow::Owned(err.client_message()),
            },
            RealmError::ActionFailed(_) => AppError::BadRequest {
                code: "ACTION_FAILED",
                message: Cow::Owned(err.client_message()),
            },
            RealmError::Chat(chat_error) => chat_error.into(),
            RealmError::Conversation(conversation_error) => {
                ChatError::from(conversation_error).into()
            }
            RealmError::Storage(error) => AppError::Internal {
                internal: Cow::Owned(error.to_string()),
            },
            RealmError::Serialization(error) => AppError::Internal {
                internal: Cow::Owned(error.to_string()),
            },
            RealmError::State(error) => error.into(),
            RealmError::Character(character_error) => character_error.into(),
            RealmError::Context(context_error) => context_error.into(),
            RealmError::Persona(persona_error) => persona_error.into(),
            RealmError::Connection(connection_error) => connection_error.into(),
        }
    }
}
