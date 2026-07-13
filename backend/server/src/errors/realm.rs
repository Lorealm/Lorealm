use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{
    CharacterError, ChatError, ConnectionError, ContextError, ConversationError, PersonaError,
    StateError, TargetType,
};

#[derive(Debug, thiserror::Error)]
pub enum RealmError {
    #[error("not playable: {0}")]
    NotPlayable(Snowflake),
    #[error("conversation {0} was not started from a realm")]
    NotRealmConversation(Snowflake),
    #[error("no agents: {0}")]
    NoAgents(Snowflake),
    #[error("not your turn: {0}")]
    NotYourTurn(Snowflake),
    #[error("malformed agent: {1} ({0})")]
    MalformedAgent(Snowflake, String),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("{0} not owned: {1}")]
    NotOwned(TargetType, Snowflake),
    #[error("realm creation: {0}")]
    Create(String),
    #[error("invalid agent: {0}")]
    InvalidAgent(String),
    #[error("invalid turn: {0}")]
    InvalidTurn(String),
    #[error("tool not bound to agent: {0}")]
    ToolNotBound(Snowflake),
    #[error("action failed: {0}")]
    ActionFailed(String),
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

impl RealmError {
    pub fn client_message(&self) -> String {
        match self {
            RealmError::NotPlayable(_) => "this realm is archived".to_string(),
            RealmError::NotRealmConversation(_) => {
                "this conversation was not started from a realm".to_string()
            }
            RealmError::NoAgents(_) => format!("realm contains no agents"),
            RealmError::NotYourTurn(_) => format!("it's not this agent's turn"),
            RealmError::MalformedAgent(_, hint) => format!("malformed agent: {hint}"),
            RealmError::NotFound(_, _) | RealmError::NotOwned(_, _) => self.to_string(),
            RealmError::Chat(chat_error) => chat_error.to_string(),
            RealmError::Conversation(conversation_error) => conversation_error.to_string(),
            RealmError::Storage(_) => format!("storage error"),
            RealmError::Serialization(error) => format!("serialization error: {error}"),
            RealmError::Create(hint) => format!("creation error: {hint}"),
            RealmError::State(state_error) => state_error.client_message(),
            RealmError::InvalidAgent(hint) => format!("invalid agent: {hint}"),
            RealmError::InvalidTurn(hint) => format!("invalid turn: {hint}"),
            RealmError::ToolNotBound(_) => "this action is not available to the agent".to_string(),
            RealmError::ActionFailed(hint) => format!("action failed: {hint}"),
            RealmError::Character(character_error) => character_error.client_message().to_string(),
            RealmError::Context(context_error) => context_error.client_message(),
            RealmError::Persona(persona_error) => persona_error.client_message(),
            RealmError::Connection(connection_error) => connection_error.to_string(),
        }
    }
}
