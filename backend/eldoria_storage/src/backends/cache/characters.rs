//! Redis-cached wrapper for `CharacterStorage`.
//!
//! Caching policy (per our earlier decision):
//!   - Read: `get_character` caches on hit under `character:{id}`.
//!   - Write: `save_character`, `delete_character`, `restore_character_version`
//!     all invalidate `character:{id}`.
//!   - Everything else (lists, versioned reads, version listings) passes
//!     through without caching. These calls are rare and cache invalidation
//!     for them isn't worth the complexity.

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
        // Uncached passthrough. Pinned reads are rare; caching them would
        // require a second key per (id, version_id) tuple and we'd rather
        // not pay for that.
        self.backend.get_character_at_version(id, version_id).await
    }

    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError> {
        // Uncached. Lists invalidate on any character write for that user,
        // which is a lot of cross-key accounting for marginal benefit.
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
