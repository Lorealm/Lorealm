use async_trait::async_trait;

use crate::{StorageError, models::{Snowflake, UserTool}};

#[async_trait]
pub trait UserToolStorage: Send + Sync {
    async fn list_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, StorageError>;
    async fn list_enabled_tools(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserTool>, StorageError>;
    async fn get_tool(&self, id: Snowflake) -> Result<Option<UserTool>, StorageError>;
    async fn save_tool(&self, tool: UserTool) -> Result<(), StorageError>;
    async fn set_tool_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError>;
    async fn delete_tool(&self, id: Snowflake) -> Result<(), StorageError>;
}
