use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Lorebook, LorebookMetadata, LorebookVersionInfo, Snowflake};
use crate::storage::LorebookStorage;

use super::CachedStorage;

#[async_trait]
impl LorebookStorage for CachedStorage {
    async fn get_lorebook(&self, id: Snowflake) -> Result<Option<Lorebook>, StorageError> {
        let key = self.key("lorebook", &id.to_string());
        self.get_through(&key, self.backend.get_lorebook(id)).await
    }

    async fn get_lorebook_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Lorebook>, StorageError> {
        let key = self.key("lorebook_ver", &format!("{id}:{version_id}"));
        self.get_through(&key, self.backend.get_lorebook_at_version(id, version_id))
            .await
    }

    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError> {
        self.backend.list_lorebooks_by_owner(user_id).await
    }

    async fn save_lorebook(&self, lorebook: &Lorebook) -> Result<Snowflake, StorageError> {
        let version_id = self.backend.save_lorebook(lorebook).await?;
        self.cache_del(&self.key("lorebook", &lorebook.id.to_string()))
            .await;
        Ok(version_id)
    }

    async fn delete_lorebook(&self, id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_lorebook(id).await?;
        if deleted {
            self.cache_del(&self.key("lorebook", &id.to_string())).await;
        }
        Ok(deleted)
    }

    async fn list_lorebook_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<LorebookVersionInfo>, StorageError> {
        self.backend.list_lorebook_versions(id).await
    }

    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let new_id = self
            .backend
            .restore_lorebook_version(id, version_id, new_version_id)
            .await?;
        self.cache_del(&self.key("lorebook", &id.to_string())).await;
        Ok(new_id)
    }

    async fn list_enabled_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let key = self.key("enabled_lorebooks", &user_id.to_string());

        if let Some(cached) = self.cache_get::<Vec<Snowflake>>(&key).await {
            return Ok(cached);
        }

        let ids = self.backend.list_enabled_lorebooks(user_id).await?;
        self.cache_set(&key, &ids).await;
        Ok(ids)
    }

    async fn set_enabled_lorebooks(
        &self,
        user_id: Snowflake,
        ordered: &[Snowflake],
    ) -> Result<(), StorageError> {
        self.backend.set_enabled_lorebooks(user_id, ordered).await?;
        self.cache_del(&self.key("enabled_lorebooks", &user_id.to_string()))
            .await;
        Ok(())
    }
}
