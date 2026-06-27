use async_trait::async_trait;

use crate::{StorageError, UserToolStorage, backends::MemoryStorage, models::{Snowflake, UserTool}};

#[async_trait]
impl UserToolStorage for MemoryStorage {
    async fn list_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, StorageError> {
        let mut out: Vec<UserTool> = self
            .user_tools
            .read()
            .await
            .values()
            .filter(|t| t.user_id == user_id)
            .cloned()
            .collect();
        out.sort_by_key(|t| t.created_at);
        Ok(out)
    }

    async fn list_enabled_tools(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserTool>, StorageError> {
        let mut out: Vec<UserTool> = self
            .user_tools
            .read()
            .await
            .values()
            .filter(|t| t.user_id == user_id && t.enabled)
            .cloned()
            .collect();
        out.sort_by_key(|t| t.created_at);
        Ok(out)
    }

    async fn get_tool(&self, id: Snowflake) -> Result<Option<UserTool>, StorageError> {
        Ok(self.user_tools.read().await.get(&id).cloned())
    }

    async fn save_tool(&self, tool: UserTool) -> Result<(), StorageError> {
        self.user_tools.write().await.insert(tool.id, tool);
        Ok(())
    }

    async fn set_tool_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        if let Some(tool) = self.user_tools.write().await.get_mut(&id) {
            tool.enabled = enabled;
        }
        Ok(())
    }

    async fn delete_tool(&self, id: Snowflake) -> Result<(), StorageError> {
        self.user_tools.write().await.remove(&id);
        Ok(())
    }
}
