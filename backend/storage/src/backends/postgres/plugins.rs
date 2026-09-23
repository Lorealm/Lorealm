use async_trait::async_trait;

use crate::{PluginStorage, StorageError, backends::PostgresStorage, models::Snowflake};

#[async_trait]
impl PluginStorage for PostgresStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<String>, StorageError> {
        let handles = sqlx::query_scalar::<_, String>(
            "SELECT handle FROM user_plugins WHERE user_id = $1 ORDER BY position",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(handles)
    }

    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO user_plugins (user_id, handle, position)
            SELECT $1, $2, COALESCE(MAX(position) + 1, 0) FROM user_plugins WHERE user_id = $1
            ON CONFLICT (user_id, handle) DO NOTHING"#,
        )
        .bind(user_id)
        .bind(handle)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn disable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM user_plugins WHERE user_id = $1 AND handle = $2")
            .bind(user_id)
            .bind(handle)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
