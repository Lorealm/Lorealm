use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    StorageError,
    models::{MessageState, Snowflake},
};

#[async_trait]
pub trait WeaverStateStorage: Send + Sync {
    /// Fetch one message's state record, if it exists.
    async fn get_message_state(
        &self,
        message_id: Snowflake,
    ) -> Result<Option<MessageState>, StorageError>;

    /// Batch-fetch records for many messages, keyed by id..
    async fn get_message_states(
        &self,
        message_ids: &[Snowflake],
    ) -> Result<HashMap<Snowflake, MessageState>, StorageError>;

    /// Persist the state produced by one assistant message's assembly.
    async fn put_message_state(&self, state: &MessageState) -> Result<(), StorageError>;
}
