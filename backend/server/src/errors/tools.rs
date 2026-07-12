use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::TargetType;

#[derive(Debug, thiserror::Error)]
pub enum UserToolError {
    #[error("user tool {1} not found")]
    NotFound(TargetType, Snowflake),
    #[error("user tool not owned by caller")]
    NotOwned,
    #[error("invalid tool name: {0}")]
    InvalidName(String),
    #[error("tool body failed to compile: {0}")]
    InvalidBody(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl UserToolError {
    pub fn client_message(&self) -> String {
        match self {
            UserToolError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            UserToolError::NotOwned => "you don't own this resource".to_string(),
            UserToolError::InvalidName(msg) => format!("failed to validate tool name: {msg}"),
            UserToolError::InvalidBody(msg) => format!("failed to validate tool body: {msg}"),
            UserToolError::Storage(storage_error) => storage_error.to_string(),
        }
    }
}
