use eldoria_storage::StorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("storage error: {0}")]
    Storage(#[from] StorageError)
}

impl StateError {
    pub fn client_message(&self) -> String {
        match self {
            StateError::Storage(storage_error) => storage_error.to_string(),
        }
    }
}