use async_trait::async_trait;

use crate::{PluginStorage, StorageError, backends::CachedStorage, models::Snowflake};

#[async_trait]
impl PluginStorage for CachedStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<String>, StorageError> {
        self.backend.list_user_plugins(user_id).await
    }

    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        self.backend.enable_user_plugin(user_id, handle).await
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
}
