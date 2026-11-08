mod auth;
mod card;
mod character;
mod chat;
mod commands;
mod config;
mod connection;
mod context;
#[cfg(test)]
mod contract;
mod conversation;
mod domain;
mod lorebook;
mod permission;
mod persona;
mod plugin;
mod realm;
mod state;
mod tokenizer;
mod tools;
mod widgets;

pub use auth::*;
pub use card::*;
pub use character::*;
pub use chat::*;
pub use commands::*;
pub use config::*;
pub use connection::*;
pub use context::*;
pub use conversation::*;
pub use domain::*;
pub use lorebook::*;
pub use permission::*;
pub use persona::*;
pub use plugin::*;
pub use realm::*;
pub use state::*;
pub use tokenizer::*;
pub use tools::*;
pub use widgets::*;

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde_json::json;
use std::{borrow::Cow, fmt::Display};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    UserWidget,
    Lorebook,
    Realm,
    RealmAgent,
    Blueprint,
    RealmVersion,
    Scenario,
    Tokenizer,
    LorebookEntry,
    TemplateNode,
    Plugin,
    Role,
    User,
}

impl TargetType {
    /// Human-readable name for client-facing messages.
    pub fn label(&self) -> &'static str {
        match self {
            TargetType::Character => "character",
            TargetType::CharacterVersion => "character version",
            TargetType::Greeting => "greeting",
            TargetType::Conversation => "conversation",
            TargetType::Message => "message",
            TargetType::ExampleMessage => "example message",
            TargetType::ContextTemplate => "context template",
            TargetType::Persona => "persona",
            TargetType::ConnectionProfile => "connection profile",
            TargetType::Secret => "API key",
            TargetType::UserTool => "tool",
            TargetType::UserWidget => "widget",
            TargetType::Lorebook => "lorebook",
            TargetType::Realm => "realm",
            TargetType::Blueprint => "blueprint",
            TargetType::RealmAgent => "agent",
            TargetType::RealmVersion => "realm version",
            TargetType::Scenario => "scenario",
            TargetType::Tokenizer => "tokenizer",
            TargetType::LorebookEntry => "lorebook entry",
            TargetType::TemplateNode => "template node",
            TargetType::Plugin => "plugin",
            TargetType::Role => "role",
            TargetType::User => "user",
        }
    }
}

impl serde::Serialize for TargetType {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
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
            TargetType::UserWidget => write!(f, "user_widget"),
            TargetType::Lorebook => write!(f, "lorebook"),
            TargetType::Realm => write!(f, "realm"),
            TargetType::Blueprint => write!(f, "blueprint"),
            TargetType::RealmAgent => write!(f, "realm_agent"),
            TargetType::RealmVersion => write!(f, "realm_version"),
            TargetType::Scenario => write!(f, "scenario"),
            TargetType::Tokenizer => write!(f, "tokenizer"),
            TargetType::LorebookEntry => write!(f, "lorebook_entry"),
            TargetType::TemplateNode => write!(f, "template_node"),
            TargetType::Plugin => write!(f, "plugin"),
            TargetType::Role => write!(f, "role"),
            TargetType::User => write!(f, "user"),
        }
    }
}

/// Application-level error type exposed through API responses.
///
/// Each variant maps to a specific HTTP status code and a client-safe message.
/// Internal details are logged server-side but never leaked to the client.
#[derive(Debug, Error)]
pub enum AppError {
    /// Produced from a [`DomainError`]. Carries that error's own code, status
    /// and optional resource pointer.
    #[error("{code}: {message}")]
    Domain {
        status: StatusCode,
        code: &'static str,
        message: Cow<'static, str>,
        details: Option<ErrorDetails>,
    },
    #[error("INTERNAL: {internal}")]
    Internal { internal: Cow<'static, str> },
}

impl AppError {
    /// Route-level failure with no domain enum behind it, f.ex. upload framing
    /// or endpoint toggles. Anything belonging to a service should implement
    /// [`DomainError`] instead.
    pub fn request(
        status: StatusCode,
        code: &'static str,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::Domain {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    /// Materializes a [`DomainError`] into its wire form.
    ///
    /// `500` variants collapse into [`AppError::Internal`] to ensure their details
    /// are logged but never reaches the client.
    ///
    /// Other 5xx keep their own code and message.
    pub fn from_domain<E: DomainError>(err: &E) -> Self {
        if err.status() == StatusCode::INTERNAL_SERVER_ERROR {
            return Self::Internal {
                internal: Cow::Owned(err.to_string()),
            };
        }

        tracing::debug!("[ERROR] {}: {err}", err.code());

        Self::Domain {
            status: err.status(),
            code: err.code(),
            message: err.message(),
            details: err.details(),
        }
    }

    /// JSON body sent to clients. Shared by the HTTP and SSE transports.
    ///
    /// Additionally logs the internal detail of [`AppError::Internal`].
    pub fn response_body(&self) -> serde_json::Value {
        if let Self::Internal { internal } = self {
            tracing::error!("[ERROR] internal: {internal}");
        }

        let mut body = json!({
            "error_code": self.error_code(),
            "message": self.client_message(),
        });

        if let Self::Domain {
            details: Some(details),
            ..
        } = self
        {
            if let Ok(value) = serde_json::to_value(details) {
                body["details"] = value;
            }
        }

        body
    }

    fn status_code(&self) -> StatusCode {
        match self {
            Self::Domain { status, .. } => *status,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_code(&self) -> &'static str {
        match self {
            Self::Domain { code, .. } => code,
            Self::Internal { .. } => "internal",
        }
    }

    /// The message included in the JSON response body.
    /// Internal errors always return a fixed generic message.
    fn client_message(&self) -> &str {
        match self {
            Self::Domain { message, .. } => message,
            Self::Internal { .. } => "An internal error occurred",
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (self.status_code(), Json(self.response_body())).into_response()
    }
}
