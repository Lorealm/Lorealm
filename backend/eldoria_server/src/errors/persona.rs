use eldoria_storage::StorageError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PersonaError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError)
}


impl PersonaError {
    pub fn client_message(&self) -> String {
        match self {
            PersonaError::Storage(storage_error) => storage_error.to_string(),
        }
    }
}