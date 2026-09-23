use async_trait::async_trait;

use crate::{PluginStorage, StorageError, backends::MemoryStorage, models::Snowflake};

#[async_trait]
impl PluginStorage for MemoryStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<String>, StorageError> {
        Ok(self
            .user_plugins
            .read()
            .await
            .get(&user_id)
            .cloned()
            .unwrap_or_default())
    }

    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        let mut plugins = self.user_plugins.write().await;
        let handles = plugins.entry(user_id).or_default();
        if !handles.iter().any(|h| h == handle) {
            handles.push(handle.to_owned());
        }
        Ok(())
    }

    async fn disable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        if let Some(handles) = self.user_plugins.write().await.get_mut(&user_id) {
            handles.retain(|h| h != handle);
        }
        Ok(())
    }

    async fn get_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let id = (user_id, handle.to_owned(), key.to_owned());
        Ok(self.plugin_data.read().await.get(&id).cloned())
    }

    async fn set_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), StorageError> {
        let id = (user_id, handle.to_owned(), key.to_owned());
        self.plugin_data.write().await.insert(id, value.to_vec());
        Ok(())
    }

    async fn delete_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<(), StorageError> {
        let id = (user_id, handle.to_owned(), key.to_owned());
        self.plugin_data.write().await.remove(&id);
        Ok(())
    }
}
