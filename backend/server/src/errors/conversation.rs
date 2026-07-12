use lorealm_storage::{StorageError, models::Snowflake};

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("conversation not found: {0}")]
    ConversationNotFound(Snowflake),
    #[error("message {0} not found in this conversation")]
    MessageNotFound(Snowflake),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl ConversationError {
    pub fn client_message(&self) -> String {
        match self {
            ConversationError::ConversationNotFound(snowflake) => {
                format!("conversation not found: {snowflake}")
            }
            ConversationError::MessageNotFound(snowflake) => {
                format!("message not found: {snowflake}")
            }
            ConversationError::Storage(storage_error) => storage_error.to_string(),
        }
    }
}
