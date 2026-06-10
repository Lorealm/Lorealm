use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    StorageError, WeaverStateStorage,
    backends::MemoryStorage,
    models::{MessageState, Snowflake},
};

#[async_trait]
impl WeaverStateStorage for MemoryStorage {
    async fn get_message_state(
        &self,
        message_id: Snowflake,
    ) -> Result<Option<MessageState>, StorageError> {
        Ok(self.message_states.read().await.get(&message_id).cloned())
    }

    async fn get_message_states(
        &self,
        message_ids: &[Snowflake],
    ) -> Result<HashMap<Snowflake, MessageState>, StorageError> {
        let guard = self.message_states.read().await;
        Ok(message_ids
            .iter()
            .filter_map(|id| guard.get(id).map(|s| (*id, s.clone())))
            .collect())
    }

    async fn put_message_state(&self, state: &MessageState) -> Result<(), StorageError> {
        self.message_states
            .write()
            .await
            .insert(state.message_id, state.clone());
        Ok(())
    }
}
