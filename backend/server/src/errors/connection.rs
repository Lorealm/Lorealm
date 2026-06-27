use lorealm_encryption::EncryptionError;
use lorealm_storage::{StorageError, models::Snowflake};

use crate::errors::TargetType;

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("{0:?} {1} not found")]
    NotFound(TargetType, Snowflake),

    #[error("{0:?} {1} is not owned by this user")]
    NotOwned(TargetType, Snowflake),

    #[error("validation failed: {0}")]
    Validation(String),

    /// Stored encrypted payload failed structural reconstruction (wrong
    /// nonce/tag size, mismatched context). Indicates corruption or
    /// tampering, not user error.
    #[error("secret {0} payload is corrupted")]
    Corrupted(Snowflake),

    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Encryption(#[from] EncryptionError),
}

impl ConnectionError {
    pub fn client_message(&self) -> String {
        match self {
            ConnectionError::NotFound(target_type, snowflake) => {
                format!("{target_type} not found: {snowflake}")
            }
            ConnectionError::NotOwned(target_type, snowflake) => {
                format!("{target_type} not owned: {snowflake}")
            }
            ConnectionError::Validation(msg) => format!("validation failed: {msg}"),
            ConnectionError::Corrupted(snowflake) => format!("secret is corrupted: {snowflake}"),
            ConnectionError::Storage(storage_error) => storage_error.to_string(),
            ConnectionError::Encryption(encryption_error) => encryption_error.to_string(),
        }
    }
}
