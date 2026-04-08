use eldoria_storage::StorageError;
use thiserror::Error;

use crate::services::permission::Permissions;

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Missing permissions: {0}")]
    Missing(Permissions),
    #[error("Storage Error: {0}")]
    Storage(#[from] StorageError),
}