use async_trait::async_trait;

use crate::{
    PluginStorage, StorageError,
    backends::CachedStorage,
    models::{PluginLog, Snowflake, UserPlugin},
};

#[async_trait]
impl PluginStorage for CachedStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<UserPlugin>, StorageError> {
        self.backend.list_user_plugins(user_id).await
    }

    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        self.backend
            .enable_user_plugin(user_id, handle, version)
            .await
    }

    async fn set_user_plugin_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        self.backend
            .set_user_plugin_version(user_id, handle, version)
            .await
    }

    async fn is_plugin_version_used(
        &self,
        handle: &str,
        version: &str,
    ) -> Result<bool, StorageError> {
        self.backend.is_plugin_version_used(handle, version).await
    }

    async fn set_user_plugin_order(
        &self,
        user_id: Snowflake,
        handles: &[String],
    ) -> Result<(), StorageError> {
        self.backend.set_user_plugin_order(user_id, handles).await
    }

    async fn disable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        self.backend.disable_user_plugin(user_id, handle).await
    }

    async fn get_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        self.backend.get_plugin_data(user_id, handle, key).await
    }

    async fn set_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), StorageError> {
        self.backend
            .set_plugin_data(user_id, handle, key, value)
            .await
    }

    async fn delete_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<(), StorageError> {
        self.backend.delete_plugin_data(user_id, handle, key).await
    }

    async fn get_plugin_settings(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Option<serde_json::Value>, StorageError> {
        self.backend.get_plugin_settings(user_id, handle).await
    }

    async fn set_plugin_settings(
        &self,
        user_id: Snowflake,
        handle: &str,
        settings: &serde_json::Value,
    ) -> Result<(), StorageError> {
        self.backend
            .set_plugin_settings(user_id, handle, settings)
            .await
    }

    async fn get_plugin_data_version(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<Option<String>, StorageError> {
        self.backend.get_plugin_data_version(user_id, handle).await
    }

    async fn set_plugin_data_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        self.backend
            .set_plugin_data_version(user_id, handle, version)
            .await
    }

    async fn append_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        logs: &[PluginLog],
        cap: u32,
    ) -> Result<(), StorageError> {
        self.backend
            .append_plugin_logs(user_id, handle, logs, cap)
            .await
    }

    async fn list_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        limit: u32,
    ) -> Result<Vec<PluginLog>, StorageError> {
        self.backend.list_plugin_logs(user_id, handle, limit).await
    }
}
