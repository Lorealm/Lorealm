use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::TargetType;

#[derive(Debug, Error)]
pub enum PersonaError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("persona not owned")]
    NotOwned,
    #[error("unsupported media type")]
    UnsupportedMediaType,
}

impl PersonaError {
    pub fn client_message(&self) -> String {
        match self {
            PersonaError::Storage(storage_error) => storage_error.to_string(),
            PersonaError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            PersonaError::NotOwned => "you do not own this persona".to_string(),
            PersonaError::UnsupportedMediaType => "unsupported media type".to_string(),
        }
    }
}
