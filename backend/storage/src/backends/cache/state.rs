use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    StorageError, WeaverStateStorage,
    backends::CachedStorage,
    models::{MessageState, Snowflake},
};

#[async_trait]
impl WeaverStateStorage for CachedStorage {
    async fn get_message_state(
        &self,
        message_id: Snowflake,
    ) -> Result<Option<MessageState>, StorageError> {
        let key = self.key("message_state", &message_id.to_string());
        self.get_through(&key, self.backend.get_message_state(message_id))
            .await
    }

    async fn get_message_states(
        &self,
        message_ids: &[Snowflake],
    ) -> Result<HashMap<Snowflake, MessageState>, StorageError> {
        let mut out = HashMap::with_capacity(message_ids.len());
        let mut misses = Vec::new();

        for &id in message_ids {
            let key = self.key("message_state", &id.to_string());
            match self.cache_get::<MessageState>(&key).await {
                Some(hit) => {
                    out.insert(id, hit);
                }
                None => misses.push(id),
            }
        }

        if !misses.is_empty() {
            for (id, state) in self.backend.get_message_states(&misses).await? {
                let key = self.key("message_state", &id.to_string());
                self.cache_set(&key, &state).await;
                out.insert(id, state);
            }
        }

        Ok(out)
    }

    async fn put_message_state(&self, state: &MessageState) -> Result<(), StorageError> {
        self.backend.put_message_state(state).await?;
        let key = self.key("message_state", &state.message_id.to_string());
        self.cache_set(&key, state).await;
        Ok(())
    }
}
