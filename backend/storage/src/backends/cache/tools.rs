use async_trait::async_trait;

use crate::{StorageError, UserToolStorage, backends::CachedStorage, models::{Snowflake, UserTool}};

#[async_trait]
impl UserToolStorage for CachedStorage {
    async fn list_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, StorageError> {
        self.backend.list_tools(user_id).await
    }

    async fn list_enabled_tools(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserTool>, StorageError> {
        self.backend.list_enabled_tools(user_id).await
    }

    async fn get_tool(&self, id: Snowflake) -> Result<Option<UserTool>, StorageError> {
        let key = self.key("user_tool", &id.to_string());
        self.get_through(&key, self.backend.get_tool(id)).await
    }

    async fn save_tool(&self, tool: UserTool) -> Result<(), StorageError> {
        let id = tool.id;
        self.backend.save_tool(tool).await?;
        self.cache_del(&self.key("user_tool", &id.to_string()))
            .await;
        Ok(())
    }

    async fn set_tool_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        self.backend.set_tool_enabled(id, enabled).await?;
        self.cache_del(&self.key("user_tool", &id.to_string()))
            .await;
        Ok(())
    }

    async fn delete_tool(&self, id: Snowflake) -> Result<(), StorageError> {
        self.backend.delete_tool(id).await?;
        self.cache_del(&self.key("user_tool", &id.to_string()))
            .await;
        Ok(())
    }
}
