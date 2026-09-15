use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{
    CharacterError, ChatError, ConnectionError, ContextError, ConversationError, DomainError,
    ErrorDetails, PersonaError, StateError, TargetType, impl_app_error, sentence,
};

#[derive(Debug, thiserror::Error)]
pub enum RealmError {
    #[error("conversation {0} was not started from a realm")]
    NotRealmConversation(Snowflake),
    #[error("no agents: {0}")]
    NoAgents(Snowflake),
    #[error("no agent's run condition passed this turn")]
    NoRunnableAgent,
    #[error("not your turn: it is \"{0}\"'s")]
    NotYourTurn(String),
    #[error("malformed agent: {1} ({0})")]
    MalformedAgent(String, String),
    #[error("no cast member with handle \"{0}\"")]
    NoSuchAgent(String),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("{0} not owned: {1}")]
    NotOwned(TargetType, Snowflake),
    #[error("invalid agent: {0}")]
    InvalidAgent(String),
    #[error("invalid turn: {0}")]
    InvalidTurn(String),
    #[error("tool not bound to agent: {0}")]
    ToolNotBound(Snowflake),
    #[error("action failed: {0}")]
    ActionFailed(String),
    #[error("invalid template: {0}")]
    InvalidTemplate(String),
    #[error("invalid scenario: {0}")]
    InvalidScenario(String),
    #[error("director failed: {0}")]
    DirectorFailed(String),
    #[error(transparent)]
    Chat(#[from] ChatError),
    #[error(transparent)]
    Conversation(#[from] ConversationError),
    #[error(transparent)]
    State(#[from] StateError),
    #[error(transparent)]
    Character(#[from] CharacterError),
    #[error(transparent)]
    Context(#[from] ContextError),
    #[error(transparent)]
    Persona(#[from] PersonaError),
    #[error(transparent)]
    Connection(#[from] ConnectionError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

impl DomainError for RealmError {
    fn code(&self) -> &'static str {
        match self {
            Self::Chat(err) => err.code(),
            Self::Conversation(err) => err.code(),
            Self::State(err) => err.code(),
            Self::Character(err) => err.code(),
            Self::Context(err) => err.code(),
            Self::Persona(err) => err.code(),
            Self::Connection(err) => err.code(),
            Self::NotRealmConversation(_) => "realm.not_realm_conversation",
            Self::NoAgents(_) => "realm.no_agents",
            Self::NoRunnableAgent => "realm.no_runnable_agent",
            Self::NotYourTurn(_) => "realm.not_your_turn",
            Self::MalformedAgent(_, _) => "realm.malformed_agent",
            Self::NoSuchAgent(_) => "realm.no_such_agent",
            Self::NotFound(_, _) | Self::NotOwned(_, _) => "realm.not_found",
            Self::InvalidAgent(_) => "realm.invalid_agent",
            Self::InvalidTurn(_) => "realm.invalid_turn",
            Self::ToolNotBound(_) => "realm.tool_not_bound",
            Self::ActionFailed(_) => "realm.action_failed",
            Self::InvalidScenario(_) => "realm.invalid_scenario",
            Self::InvalidTemplate(_) => "realm.invalid_template",
            Self::DirectorFailed(_) => "realm.director_failed",
            Self::Storage(_) | Self::Serialization(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Chat(err) => err.status(),
            Self::Conversation(err) => err.status(),
            Self::State(err) => err.status(),
            Self::Character(err) => err.status(),
            Self::Context(err) => err.status(),
            Self::Persona(err) => err.status(),
            Self::Connection(err) => err.status(),
            Self::NotFound(_, _) | Self::NotOwned(_, _) => StatusCode::NOT_FOUND,
            Self::NotYourTurn(_) | Self::NoRunnableAgent => StatusCode::CONFLICT,
            Self::NotRealmConversation(_)
            | Self::NoAgents(_)
            | Self::MalformedAgent(_, _)
            | Self::NoSuchAgent(_)
            | Self::InvalidAgent(_)
            | Self::InvalidTurn(_)
            | Self::ToolNotBound(_)
            | Self::ActionFailed(_)
            | Self::InvalidScenario(_)
            | Self::InvalidTemplate(_)
            | Self::DirectorFailed(_) => StatusCode::BAD_REQUEST,
            Self::Storage(_) | Self::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Chat(err) => err.message(),
            Self::Conversation(err) => err.message(),
            Self::State(err) => err.message(),
            Self::Character(err) => err.message(),
            Self::Context(err) => err.message(),
            Self::Persona(err) => err.message(),
            Self::Connection(err) => err.message(),
            Self::NotRealmConversation(_) => {
                "This conversation was not started from a realm.".into()
            }
            Self::NoAgents(_) => "This realm has no agents yet.".into(),
            Self::NoRunnableAgent => {
                "No agent could run this turn — every run condition was false.".into()
            }
            Self::NotYourTurn(_) => "It isn't this agent's turn.".into(),
            Self::NoSuchAgent(handle) => {
                Cow::Owned(format!("No cast member is called \"{handle}\"."))
            }
            Self::MalformedAgent(_, hint) => {
                sentence(format!("This agent is misconfigured: {hint}"))
            }
            Self::NotFound(target, _) | Self::NotOwned(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::InvalidAgent(hint) => sentence(format!("Invalid agent: {hint}")),
            Self::InvalidTurn(hint) => sentence(format!("Invalid turn: {hint}")),
            Self::ToolNotBound(_) => "This action isn't available to the agent.".into(),
            Self::ActionFailed(hint) => sentence(format!("Action failed: {hint}")),
            Self::InvalidScenario(hint) => sentence(format!("Invalid scenario: {hint}")),
            Self::InvalidTemplate(hint) => sentence(format!("Invalid template: {hint}")),
            Self::DirectorFailed(hint) => {
                sentence(format!("The realm could not decide who acts next: {hint}"))
            }
            Self::Storage(_) | Self::Serialization(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::Chat(err) => err.details(),
            Self::Conversation(err) => err.details(),
            Self::State(err) => err.details(),
            Self::Character(err) => err.details(),
            Self::Context(err) => err.details(),
            Self::Persona(err) => err.details(),
            Self::Connection(err) => err.details(),
            Self::NotFound(target, id) | Self::NotOwned(target, id) => {
                Some(ErrorDetails::target(*target, *id))
            }
            Self::NotRealmConversation(id) => {
                Some(ErrorDetails::target(TargetType::Conversation, *id))
            }
            Self::NoAgents(id) => Some(ErrorDetails::target(TargetType::Realm, *id)),
            Self::NotYourTurn(_) | Self::MalformedAgent(_, _) | Self::NoSuchAgent(_) => {
                Some(ErrorDetails::resource(TargetType::RealmAgent))
            }
            Self::ToolNotBound(id) => Some(ErrorDetails::target(TargetType::UserTool, *id)),
            Self::InvalidAgent(_) => Some(ErrorDetails::resource(TargetType::RealmAgent)),
            Self::InvalidScenario(_) => Some(ErrorDetails::resource(TargetType::Scenario)),
            Self::NoRunnableAgent
            | Self::InvalidTurn(_)
            | Self::ActionFailed(_)
            | Self::InvalidTemplate(_)
            | Self::DirectorFailed(_) => Some(ErrorDetails::resource(TargetType::Realm)),
            Self::Storage(_) | Self::Serialization(_) => None,
        }
    }
}

impl_app_error!(RealmError);
