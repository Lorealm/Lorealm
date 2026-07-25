use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::TargetType;

#[derive(Debug, Error)]
pub enum UserWidgetError {
    #[error("widget {1} not found")]
    NotFound(TargetType, Snowflake),
    #[error("widget not owned by caller")]
    NotOwned,
    #[error("invalid widget: {0}")]
    Invalid(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl UserWidgetError {
    pub fn client_message(&self) -> String {
        match self {
            UserWidgetError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            UserWidgetError::NotOwned => "you don't own this resource".to_string(),
            UserWidgetError::Invalid(msg) => format!("invalid widget: {msg}"),
            UserWidgetError::Storage(storage_error) => storage_error.to_string(),
        }
    }
}
