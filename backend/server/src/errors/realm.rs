use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::{ChatError, TargetType};

#[derive(Debug, thiserror::Error)]
pub enum RealmError {
    #[error("not active: {0}")]
    NotActive(Snowflake),
    #[error("no agents: {0}")]
    NoAgents(Snowflake),
    #[error("not your turn: {0}")]
    NotYourTurn(Snowflake),
    #[error("tick conflict: {0}")]
    TickConflict(Snowflake),
    #[error("malformed agent: {0}")]
    MalformedAgent(Snowflake),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("{0} not owned: {1}")]
    NotOwned(TargetType, Snowflake),
    #[error("corrupt state: {0}")]
    StateCorrupt(serde_json::Error),
    #[error(transparent)]
    Chat(#[from] ChatError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Serialization(#[from] serde_json::Error)
}