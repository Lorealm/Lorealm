use async_trait::async_trait;

use crate::{
    PluginStorage, StorageError,
    backends::PostgresStorage,
    models::{PluginLog, Snowflake, UserPlugin},
};

#[async_trait]
impl PluginStorage for PostgresStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<UserPlugin>, StorageError> {
        let plugins = sqlx::query_as::<_, UserPlugin>(
            "SELECT handle, version FROM user_plugins WHERE user_id = $1 ORDER BY position",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(plugins)
    }

    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO user_plugins (user_id, handle, version, position)
            SELECT $1, $2, $3, COALESCE(MAX(position) + 1, 0) FROM user_plugins WHERE user_id = $1
            ON CONFLICT (user_id, handle) DO NOTHING"#,
        )
        .bind(user_id)
        .bind(handle)
        .bind(version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn set_user_plugin_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("UPDATE user_plugins SET version = $3 WHERE user_id = $1 AND handle = $2")
            .bind(user_id)
            .bind(handle)
            .bind(version)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn is_plugin_version_used(
        &self,
        handle: &str,
        version: &str,
    ) -> Result<bool, StorageError> {
        let used = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (SELECT 1 FROM user_plugins WHERE handle = $1 AND version = $2)",
        )
        .bind(handle)
        .bind(version)
        .fetch_one(&self.pool)
        .await?;
        Ok(used)
    }

    async fn set_user_plugin_order(
        &self,
        user_id: Snowflake,
        handles: &[String],
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        for (position, handle) in handles.iter().enumerate() {
            sqlx::query("UPDATE user_plugins SET position = $3 WHERE user_id = $1 AND handle = $2")
                .bind(user_id)
                .bind(handle)
                .bind(position as i32)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
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

    async fn get_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let value = sqlx::query_scalar::<_, Vec<u8>>(
            "SELECT value FROM plugin_data WHERE user_id = $1 AND handle = $2 AND key = $3",
        )
        .bind(user_id)
        .bind(handle)
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;
        Ok(value)
    }

    async fn set_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO plugin_data (user_id, handle, key, value)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, handle, key) DO UPDATE SET
                value = EXCLUDED.value,
                updated_at = now()"#,
        )
        .bind(user_id)
        .bind(handle)
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn delete_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM plugin_data WHERE user_id = $1 AND handle = $2 AND key = $3")
            .bind(user_id)
            .bind(handle)
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_plugin_settings(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Option<serde_json::Value>, StorageError> {
        let settings = sqlx::query_scalar::<_, serde_json::Value>(
            "SELECT settings FROM plugin_settings WHERE user_id = $1 AND handle = $2",
        )
        .bind(user_id)
        .bind(handle)
        .fetch_optional(&self.pool)
        .await?;
        Ok(settings)
    }

    async fn set_plugin_settings(
        &self,
        user_id: Snowflake,
        handle: &str,
        settings: &serde_json::Value,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO plugin_settings (user_id, handle, settings)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, handle) DO UPDATE SET
                settings = EXCLUDED.settings,
                updated_at = now()"#,
        )
        .bind(user_id)
        .bind(handle)
        .bind(settings)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_plugin_data_version(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Option<String>, StorageError> {
        let version = sqlx::query_scalar::<_, String>(
            "SELECT version FROM plugin_data_versions WHERE user_id = $1 AND handle = $2",
        )
        .bind(user_id)
        .bind(handle)
        .fetch_optional(&self.pool)
        .await?;
        Ok(version)
    }

    async fn set_plugin_data_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO plugin_data_versions (user_id, handle, version)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, handle) DO UPDATE SET version = EXCLUDED.version"#,
        )
        .bind(user_id)
        .bind(handle)
        .bind(version)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn append_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        logs: &[PluginLog],
        cap: u32,
    ) -> Result<(), StorageError> {
        let mut tx = self.pool.begin().await?;
        for log in logs {
            sqlx::query(
                r#"INSERT INTO plugin_logs (id, user_id, handle, level, source, message, created_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
            )
            .bind(log.id)
            .bind(log.user_id)
            .bind(&log.handle)
            .bind(log.level)
            .bind(log.source)
            .bind(&log.message)
            .bind(log.created_at)
            .execute(&mut *tx)
            .await?;
        }
        sqlx::query(
            r#"DELETE FROM plugin_logs WHERE user_id = $1 AND handle = $2 AND id < (
                SELECT id FROM plugin_logs WHERE user_id = $1 AND handle = $2
                ORDER BY id DESC OFFSET GREATEST($3 - 1, 0) LIMIT 1
            )"#,
        )
        .bind(user_id)
        .bind(handle)
        .bind(i64::from(cap))
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    async fn list_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        limit: u32,
    ) -> Result<Vec<PluginLog>, StorageError> {
        let logs = sqlx::query_as::<_, PluginLog>(
            "SELECT * FROM plugin_logs WHERE user_id = $1 AND handle = $2 ORDER BY id DESC LIMIT $3",
        )
        .bind(user_id)
        .bind(handle)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await?;
        Ok(logs)
    }
}
