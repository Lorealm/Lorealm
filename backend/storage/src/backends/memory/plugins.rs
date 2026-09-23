use async_trait::async_trait;

use crate::{
    PluginStorage, StorageError,
    backends::MemoryStorage,
    models::{PluginLog, Snowflake, UserPlugin},
};

#[async_trait]
impl PluginStorage for MemoryStorage {
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<UserPlugin>, StorageError> {
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
        version: &str,
    ) -> Result<(), StorageError> {
        let mut plugins = self.user_plugins.write().await;
        let enabled = plugins.entry(user_id).or_default();
        if !enabled.iter().any(|p| p.handle == handle) {
            enabled.push(UserPlugin {
                handle: handle.to_owned(),
                version: version.to_owned(),
            });
        }
        Ok(())
    }

    async fn set_user_plugin_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError> {
        if let Some(enabled) = self.user_plugins.write().await.get_mut(&user_id) {
            for plugin in enabled.iter_mut().filter(|p| p.handle == handle) {
                plugin.version = version.to_owned();
            }
        }
        Ok(())
    }

    async fn is_plugin_version_used(
        &self,
        handle: &str,
        version: &str,
    ) -> Result<bool, StorageError> {
        Ok(self
            .user_plugins
            .read()
            .await
            .values()
            .flatten()
            .any(|p| p.handle == handle && p.version == version))
    }

    async fn set_user_plugin_order(
        &self,
        user_id: Snowflake,
        handles: &[String],
    ) -> Result<(), StorageError> {
        if let Some(enabled) = self.user_plugins.write().await.get_mut(&user_id) {
            enabled.sort_by_key(|p| handles.iter().position(|h| *h == p.handle));
        }
        Ok(())
    }

    async fn disable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError> {
        if let Some(enabled) = self.user_plugins.write().await.get_mut(&user_id) {
            enabled.retain(|p| p.handle != handle);
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

    async fn append_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        logs: &[PluginLog],
        cap: u32,
    ) -> Result<(), StorageError> {
        let mut all = self.plugin_logs.write().await;
        let log = all.entry((user_id, handle.to_owned())).or_default();
        log.extend_from_slice(logs);
        let excess = log.len().saturating_sub(cap as usize);
        log.drain(..excess);
        Ok(())
    }

    async fn list_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        limit: u32,
    ) -> Result<Vec<PluginLog>, StorageError> {
        let all = self.plugin_logs.read().await;
        let Some(log) = all.get(&(user_id, handle.to_owned())) else {
            return Ok(Vec::new());
        };
        Ok(log.iter().rev().take(limit as usize).cloned().collect())
    }
}
