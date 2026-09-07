use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{
    StorageError,
    models::{ImportKind, Snowflake},
};

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
    #[error("not your turn: {0}")]
    NotYourTurn(Snowflake),
    #[error("malformed agent: {1} ({0})")]
    MalformedAgent(Snowflake, String),
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
    #[error("{} {1} is not imported into the realm", .0.as_str())]
    NotImported(ImportKind, Snowflake),
    #[error("{} {1} is still referenced by agent \"{2}\"", .0.as_str())]
    ImportInUse(ImportKind, Snowflake, String),
    #[error("action failed: {0}")]
    ActionFailed(String),
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
            Self::NotFound(_, _) | Self::NotOwned(_, _) => "realm.not_found",
            Self::InvalidAgent(_) => "realm.invalid_agent",
            Self::InvalidTurn(_) => "realm.invalid_turn",
            Self::ToolNotBound(_) => "realm.tool_not_bound",
            Self::NotImported(_, _) => "realm.not_imported",
            Self::ImportInUse(_, _, _) => "realm.import_in_use",
            Self::ActionFailed(_) => "realm.action_failed",
            Self::InvalidScenario(_) => "realm.invalid_scenario",
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
            Self::NotYourTurn(_)
            | Self::NoRunnableAgent
            | Self::ImportInUse(_, _, _)
            | Self::NotImported(_, _) => StatusCode::CONFLICT,
            Self::NotRealmConversation(_)
            | Self::NoAgents(_)
            | Self::MalformedAgent(_, _)
            | Self::InvalidAgent(_)
            | Self::InvalidTurn(_)
            | Self::ToolNotBound(_)
            | Self::ActionFailed(_)
            | Self::InvalidScenario(_)
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
            Self::MalformedAgent(_, hint) => {
                sentence(format!("This agent is misconfigured: {hint}"))
            }
            Self::NotFound(target, _) | Self::NotOwned(target, _) => {
                Cow::Owned(format!("That {} no longer exists.", target.label()))
            }
            Self::InvalidAgent(hint) => sentence(format!("Invalid agent: {hint}")),
            Self::InvalidTurn(hint) => sentence(format!("Invalid turn: {hint}")),
            Self::ToolNotBound(_) => "This action isn't available to the agent.".into(),
            Self::NotImported(kind, _) => Cow::Owned(format!(
                "This {} isn't imported into the realm — import it first.",
                kind.as_str()
            )),
            Self::ImportInUse(kind, _, agent) => Cow::Owned(format!(
                "This {} is still used by agent \"{agent}\" — unbind it there first.",
                kind.as_str()
            )),
            Self::ActionFailed(hint) => sentence(format!("Action failed: {hint}")),
            Self::InvalidScenario(hint) => sentence(format!("Invalid scenario: {hint}")),
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
            Self::NotRealmConversation(id) | Self::NotYourTurn(id) => {
                Some(ErrorDetails::target(TargetType::Conversation, *id))
            }
            Self::NoAgents(id) => Some(ErrorDetails::target(TargetType::Realm, *id)),
            Self::MalformedAgent(id, _) => Some(ErrorDetails::target(TargetType::RealmAgent, *id)),
            Self::ToolNotBound(id) => Some(ErrorDetails::target(TargetType::UserTool, *id)),
            Self::NotImported(kind, id) | Self::ImportInUse(kind, id, _) => {
                Some(ErrorDetails::target(import_target(*kind), *id))
            }
            Self::InvalidAgent(_) => Some(ErrorDetails::resource(TargetType::RealmAgent)),
            Self::InvalidScenario(_) => Some(ErrorDetails::resource(TargetType::Scenario)),
            Self::NoRunnableAgent
            | Self::InvalidTurn(_)
            | Self::ActionFailed(_)
            | Self::DirectorFailed(_) => Some(ErrorDetails::resource(TargetType::Realm)),
            Self::Storage(_) | Self::Serialization(_) => None,
        }
    }
}

fn import_target(kind: ImportKind) -> TargetType {
    match kind {
        ImportKind::Character => TargetType::Character,
        ImportKind::Lorebook => TargetType::Lorebook,
        ImportKind::Template => TargetType::ContextTemplate,
        ImportKind::Tool => TargetType::UserTool,
        ImportKind::Persona => TargetType::Persona,
        ImportKind::ConnectionProfile => TargetType::ConnectionProfile,
        ImportKind::Widget => TargetType::UserWidget,
    }
}

impl_app_error!(RealmError);
