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
}
