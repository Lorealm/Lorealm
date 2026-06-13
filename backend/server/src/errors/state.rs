use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::errors::TargetType;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("no active context template")]
    NoActiveTemplate,
    #[error("{0} not found: {1}")]
    NotFound(TargetType, Snowflake),
    #[error("template conversion: {0}")]
    TemplateConversion(String),
    #[error("no active persona")]
    NoActivePersona,
    #[error("no active connection profile")]
    NoActiveConnectionProfile,
}

impl StateError {
    pub fn client_message(&self) -> String {
        match self {
            StateError::Storage(storage_error) => storage_error.to_string(),
            StateError::NoActiveTemplate => "no active context template".to_string(),
            StateError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            StateError::TemplateConversion(msg) => msg.to_string(),
            StateError::NoActivePersona => "no active persona".to_string(),
            StateError::NoActiveConnectionProfile => "no active connection template".to_string(),
        }
    }
}
