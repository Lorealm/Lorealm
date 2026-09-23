use async_trait::async_trait;

use crate::{StorageError, models::Snowflake};

#[async_trait]
pub trait PluginStorage: Send + Sync {
    /// Handles of the user's enabled plugins, in load order.
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<String>, StorageError>;
    /// Appends `handle` to the user's load order. No-op if already enabled.
    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError>;
    async fn disable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
    ) -> Result<(), StorageError>;
    async fn get_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<Option<Vec<u8>>, StorageError>;
    /// Inserts or replaces the value under `key`.
    async fn set_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
        value: &[u8],
    ) -> Result<(), StorageError>;
    async fn delete_plugin_data(
        &self,
        user_id: Snowflake,
        handle: &str,
        key: &str,
    ) -> Result<(), StorageError>;
}
