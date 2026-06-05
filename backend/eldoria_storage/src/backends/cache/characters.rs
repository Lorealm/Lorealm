use crate::backends::CachedStorage;
use crate::error::StorageError;
use crate::models::{
    Character, CharacterSummary, CharacterVersionInfo, Snowflake, VersionedSaveMode,
};
use crate::storage::CharacterStorage;
use async_trait::async_trait;

impl CachedStorage {
    fn character_key(&self, id: Snowflake) -> String {
        self.key("character", &id.to_string())
    }
}

#[async_trait]
impl CharacterStorage for CachedStorage {
    async fn list_characters(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<CharacterSummary>, StorageError> {
        self.backend.list_characters(user_id).await
    }
    async fn get_character(&self, id: Snowflake) -> Result<Option<Character>, StorageError> {
        let key = self.character_key(id);
        self.get_through(&key, self.backend.get_character(id)).await
    }

    async fn get_character_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Character>, StorageError> {
        self.backend.get_character_at_version(id, version_id).await
    }

    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError> {
        self.backend.list_characters_by_owner(user_id).await
    }

    async fn save_character(
        &self,
        character: &Character,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        let result = self.backend.save_character(character, mode).await?;
        self.cache_del(&self.character_key(character.id)).await;
        Ok(result)
    }

    async fn delete_character(&self, id: Snowflake) -> Result<bool, StorageError> {
        let result = self.backend.delete_character(id).await?;
        self.cache_del(&self.character_key(id)).await;
        Ok(result)
    }

    async fn list_character_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, StorageError> {
        self.backend.list_character_versions(id).await
    }

    async fn restore_character_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let result = self
            .backend
            .restore_character_version(id, version_id, new_version_id)
            .await?;
        self.cache_del(&self.character_key(id)).await;
        Ok(result)
    }

    async fn get_latest_version(&self, id: Snowflake) -> Result<Snowflake, StorageError> {
        self.backend.get_latest_version(id).await
    }
}
