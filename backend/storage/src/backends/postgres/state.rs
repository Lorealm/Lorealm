use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    StorageError, WeaverStateStorage,
    backends::PostgresStorage,
    models::{MessageState, MessageStateRow, Snowflake},
};

#[async_trait]
impl WeaverStateStorage for PostgresStorage {
    async fn get_message_state(
        &self,
        message_id: Snowflake,
    ) -> Result<Option<MessageState>, StorageError> {
        let row = sqlx::query_as::<_, MessageStateRow>(
            "SELECT * FROM weaver_message_state WHERE message_id = $1",
        )
        .bind(message_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    async fn get_message_states(
        &self,
        message_ids: &[Snowflake],
    ) -> Result<HashMap<Snowflake, MessageState>, StorageError> {
        let rows = sqlx::query_as::<_, MessageStateRow>(
            "SELECT * FROM weaver_message_state WHERE message_id = ANY($1)",
        )
        .bind(message_ids)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|r| {
                let s: MessageState = r.into();
                (s.message_id, s)
            })
            .collect())
    }

    async fn put_message_state(&self, state: &MessageState) -> Result<(), StorageError> {
        sqlx::query(
            "INSERT INTO weaver_message_state (message_id, activation, persistent_delta)
             VALUES ($1, $2, $3)",
        )
        .bind(state.message_id)
        .bind(sqlx::types::Json(&state.activation))
        .bind(sqlx::types::Json(&state.persistent))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
