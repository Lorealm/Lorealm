use async_trait::async_trait;

use crate::{
    StorageError,
    models::{PluginLog, Snowflake},
};

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
    /// Sets the user's load order. `handles` must be exactly the enabled plugins.
    async fn set_user_plugin_order(
        &self,
        user_id: Snowflake,
        handles: &[String],
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
    /// Appends `logs` to one user's log for `handle`, then drops all but the
    /// newest `cap` entries.
    async fn append_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        logs: &[PluginLog],
        cap: u32,
    ) -> Result<(), StorageError>;
    /// Newest first.
    async fn list_plugin_logs(
        &self,
        user_id: Snowflake,
        handle: &str,
        limit: u32,
    ) -> Result<Vec<PluginLog>, StorageError>;
}
