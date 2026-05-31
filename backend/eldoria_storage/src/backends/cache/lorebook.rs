//! Redis cache wrapper for `LorebookStorage` and `LorebookSessionStorage`.
//!
//! Caches point reads and invalidates on writes. List queries pass straight
//! through — list invalidation is not worth the complexity here.
//!
//! REQUIRED: the `Storage` supertrait must include `LorebookStorage` and
//! `LorebookSessionStorage` so `self.backend` exposes these methods, e.g.:
//!
//! ```ignore
//! pub trait Storage:
//!     CharacterStorage + LorebookStorage + LorebookSessionStorage + Send + Sync
//! {}
//! ```
//!
//! ASSUMPTION: `Snowflake` implements `Display` (used for cache keys).

use async_trait::async_trait;

use crate::error::StorageError;
use crate::models::{Lorebook, LorebookMetadata, LorebookSession, LorebookVersionInfo, Snowflake};
use crate::storage::{LorebookSessionStorage, LorebookStorage};

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
        // Snapshots are immutable, so this is safe to cache under a compound key.
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
}

#[async_trait]
impl LorebookSessionStorage for CachedStorage {
    async fn get_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<LorebookSession>, StorageError> {
        let key = self.key("lb_session", &conversation_id.to_string());
        self.get_through(&key, self.backend.get_session(conversation_id))
            .await
    }

    async fn save_lorebook_session(&self, session: &LorebookSession) -> Result<(), StorageError> {
        self.backend.save_lorebook_session(session).await?;
        // Write-through: sessions are read every turn, so keep the entry warm
        // rather than just deleting it.
        let key = self.key("lb_session", &session.conversation_id.to_string());
        self.cache_set(&key, session).await;
        Ok(())
    }

    async fn delete_lorebook_session(&self, conversation_id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_lorebook_session(conversation_id).await?;
        self.cache_del(&self.key("lb_session", &conversation_id.to_string()))
            .await;
        Ok(deleted)
    }
}
