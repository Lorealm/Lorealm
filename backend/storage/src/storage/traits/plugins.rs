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
}
