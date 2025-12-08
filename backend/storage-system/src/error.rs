use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Backend not available: {0}")]
    BackendUnavailable(String),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    #[error("Connection Error: {0}")]
    ConnectionError(String),
    #[error("Not Implemented Error: {0}")]
    NotImplemented(String),
    #[error("Permission Denied: {0}")]
    PermissionDenied(String),
}
