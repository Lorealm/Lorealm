use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    StorageError, WeaverStateStorage,
    backends::SqliteStorage,
    models::{MessageState, MessageStateRow, Snowflake},
};

#[async_trait]
impl WeaverStateStorage for SqliteStorage {
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
        // SQLite cannot bind an array, so the id list travels as a JSON array and
        // is unnested with json_each.
        let ids = serde_json::to_string(message_ids)?;

        let rows = sqlx::query_as::<_, MessageStateRow>(
            "SELECT * FROM weaver_message_state
             WHERE message_id IN (SELECT value FROM json_each($1))",
        )
        .bind(&ids)
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
             VALUES ($1, $2, $3)
             ON CONFLICT (message_id) DO UPDATE
             SET activation = EXCLUDED.activation,
                 persistent_delta = EXCLUDED.persistent_delta",
        )
        .bind(state.message_id)
        .bind(sqlx::types::Json(&state.activation))
        .bind(sqlx::types::Json(&state.persistent))
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
