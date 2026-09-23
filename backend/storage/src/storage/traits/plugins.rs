use async_trait::async_trait;

use crate::{
    StorageError,
    models::{PluginLog, Snowflake, UserPlugin},
};

#[async_trait]
pub trait PluginStorage: Send + Sync {
    /// The user's enabled plugins, in load order.
    async fn list_user_plugins(&self, user_id: Snowflake) -> Result<Vec<UserPlugin>, StorageError>;
    /// Appends `handle` at `version` to the user's load order. No-op if
    /// already enabled.
    async fn enable_user_plugin(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError>;
    /// Moves an enabled plugin to another version.
    async fn set_user_plugin_version(
        &self,
        user_id: Snowflake,
        handle: &str,
        version: &str,
    ) -> Result<(), StorageError>;
    /// Whether any user has `handle` enabled at `version`.
    async fn is_plugin_version_used(
        &self,
        handle: &str,
        version: &str,
    ) -> Result<bool, StorageError>;
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
