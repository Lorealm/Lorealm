use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{
    ConversationError, DomainError, ErrorDetails, StateError, TargetType, UserToolError,
    impl_app_error, sentence,
};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("tool error: {0}")]
    Tool(#[from] UserToolError),
    #[error("conversation error: {0}")]
    Conversation(#[from] ConversationError),
    #[error("state error: {0}")]
    State(#[from] StateError),
    #[error("conversation {0} not found")]
    NotFound(Snowflake),
    #[error("conversation {0} not owned by caller")]
    NotOwned(Snowflake),
    /// Realm playthroughs advance through the grant-checked turn pipeline.
    #[error("conversation {0} is a realm playthrough")]
    RealmConversation(Snowflake),
    #[error("'{0}' is not callable as a command")]
    NotACommand(String),
    #[error("argument '{name}': {reason}")]
    BadArgument { name: String, reason: String },
    #[error("command '{name}' failed: {reason}")]
    Failed { name: String, reason: String },
    /// The user stopped it. Not a failure of the body.
    #[error("command '{0}' was cancelled")]
    Cancelled(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for CommandError {
    fn code(&self) -> &'static str {
        match self {
            Self::Tool(e) => e.code(),
            Self::Conversation(e) => e.code(),
            Self::State(e) => e.code(),
            Self::NotFound(_) | Self::NotOwned(_) => "command.not_found",
            Self::RealmConversation(_) => "command.realm_conversation",
            Self::NotACommand(_) => "command.not_a_command",
            Self::BadArgument { .. } => "command.bad_argument",
            Self::Failed { .. } => "command.failed",
            Self::Cancelled(_) => "command.cancelled",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Tool(e) => e.status(),
            Self::Conversation(e) => e.status(),
            Self::State(e) => e.status(),
            Self::NotFound(_) | Self::NotOwned(_) => StatusCode::NOT_FOUND,
            Self::RealmConversation(_) | Self::NotACommand(_) => StatusCode::CONFLICT,
            Self::BadArgument { .. } => StatusCode::BAD_REQUEST,
            // The body itself is at fault, not the request that ran it.
            Self::Failed { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            // The client asked for this, so it is not an error condition.
            Self::Cancelled(_) => StatusCode::from_u16(499).expect("valid status"),
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Tool(e) => e.message(),
            Self::Conversation(e) => e.message(),
            Self::State(e) => e.message(),
            Self::NotFound(_) | Self::NotOwned(_) => "That conversation no longer exists.".into(),
            Self::RealmConversation(_) => "Commands cannot run in a realm playthrough yet.".into(),
            Self::NotACommand(name) => sentence(format!("'{name}' is a tool, not a command")),
            Self::BadArgument { name, reason } => sentence(format!("Argument '{name}': {reason}")),
            Self::Failed { name, reason } => sentence(format!("Command /{name} failed: {reason}")),
            Self::Cancelled(name) => sentence(format!("Command /{name} was stopped")),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::Tool(e) => e.details(),
            Self::Conversation(e) => e.details(),
            Self::State(e) => e.details(),
            Self::NotFound(id) | Self::NotOwned(id) | Self::RealmConversation(id) => {
                Some(ErrorDetails::target(TargetType::Conversation, *id))
            }
            Self::NotACommand(_)
            | Self::BadArgument { .. }
            | Self::Failed { .. }
            | Self::Cancelled(_) => Some(ErrorDetails::resource(TargetType::UserTool)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(CommandError);
