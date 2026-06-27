use lorealm_storage::StorageError;
use thiserror::Error;

use crate::services::permission::Permissions;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Missing permissions: {0}")]
    Missing(Permissions),
    #[error("Storage Error: {0}")]
    Storage(#[from] StorageError),
}

impl PermissionError {
    pub fn client_message(&self) -> &'static str {
        match self {
            PermissionError::Missing(_) => "You are not allowed to perform this action.",
            PermissionError::Storage(_) => "Unknown error",
        }
    }
}
