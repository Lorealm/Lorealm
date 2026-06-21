use lorealm_storage::{StorageError, models::Snowflake};

#[derive(Debug, thiserror::Error)]
pub enum TokenizerError {
    #[error("tokenizer {0} not found")]
    NotFound(Snowflake),
    #[error("not permitted to modify tokenizer {0}")]
    Forbidden(Snowflake),
    #[error("invalid tokenizer data: {0}")]
    InvalidData(String),
    #[error("invalid tokenizer source: {0}")]
    InvalidSource(String),
    #[error("failed to fetch tokenizer: {0}")]
    Fetch(String),
    #[error("failed to tokenize: {0}")]
    Tokenize(String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl TokenizerError {
    pub fn client_message(&self) -> String {
        match self {
            TokenizerError::NotFound(snowflake) => format!("tokenizer not found: {snowflake}"),
            TokenizerError::Forbidden(snowflake) => {
                format!("insufficient permissions for tokenizer: {snowflake}")
            }
            TokenizerError::Storage(storage_error) => storage_error.to_string(),
            TokenizerError::InvalidData(_) => "the imported tokenizer was invalid".to_owned(),
            TokenizerError::InvalidSource(src) => format!("the source: '{src}' is invalid"),
            TokenizerError::Fetch(msg) => format!("failed to fetch tokenizer: {msg}"),
            TokenizerError::Tokenize(msg) => format!("failed to tokenize: {msg}"),
        }
    }
}
