//! In-memory implementation of `LorebookStorage` and `LorebookSessionStorage`.
//!
//! ASSUMPTION: `MemoryStorage` uses `std::sync::RwLock` (hence `.read().unwrap()`
//! / `.write().unwrap()`). If you're on `parking_lot`, drop the `.unwrap()`s; on
//! `tokio::sync::RwLock`, add `.await` and adjust. No guard is ever held across
//! an `.await`, so a sync lock is correct here.
//!
//! Two additions are required elsewhere:
//!
//! 1. Add these fields to `MemoryStorage` (it keeps deriving `Default`):
//!
//! ```ignore
//! pub(crate) lorebooks: RwLock<HashMap<Snowflake, Lorebook>>,
//! pub(crate) lorebook_versions: RwLock<HashMap<Snowflake, StoredLorebookVersion>>, // key: version_id
//! pub(crate) lorebook_sessions: RwLock<HashMap<Snowflake, LorebookSession>>,        // key: conversation_id
//! ```
//!
//! 2. The `StoredLorebookVersion` type below (place it wherever `StoredVersion`
//!    lives).

use async_trait::async_trait;
use chrono::Utc;

use crate::error::StorageError;
use crate::models::{
    Lorebook, LorebookMetadata, LorebookSession, LorebookVersionInfo, Snowflake,
    extract_book_version,
};
use crate::storage::{LorebookSessionStorage, LorebookStorage};

use super::MemoryStorage;

/// A stored snapshot of a lorebook aggregate at save time. The in-memory
/// analogue of the Postgres `lorebook_versions.snapshot` column.
#[derive(Debug, Clone)]
pub struct StoredLorebookVersion {
    pub lorebook_id: Snowflake,
    pub version_num: i32,
    pub book_version: String,
    pub snapshot: Lorebook,
    pub created_at: chrono::DateTime<Utc>,
}

#[async_trait]
impl LorebookStorage for MemoryStorage {
    async fn get_lorebook(&self, id: Snowflake) -> Result<Option<Lorebook>, StorageError> {
        Ok(self.lorebooks.read().await.get(&id).cloned())
    }

    async fn get_lorebook_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Lorebook>, StorageError> {
        let versions = self.lorebook_versions.read().await;
        match versions.get(&version_id) {
            Some(v) if v.lorebook_id == id => {
                let mut book = v.snapshot.clone();
                book.current_version_id = version_id;
                Ok(Some(book))
            }
            _ => Ok(None),
        }
    }

    async fn list_lorebooks_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<LorebookMetadata>, StorageError> {
        let books: Vec<Lorebook> = self
            .lorebooks
            .read()
            .await
            .values()
            .filter(|b| b.owner_id == user_id)
            .map(|b| {
                // Heads only — match the Postgres backend's light list payload.
                let mut light = b.clone();
                light.entries.clear();
                light
            })
            .collect();
        Ok(books.iter().map(LorebookMetadata::from).collect())
    }

    async fn save_lorebook(&self, lorebook: &Lorebook) -> Result<Snowflake, StorageError> {
        let mut books = self.lorebooks.write().await;
        let mut versions = self.lorebook_versions.write().await;

        let next_num = versions
            .values()
            .filter(|v| v.lorebook_id == lorebook.id)
            .map(|v| v.version_num)
            .max()
            .unwrap_or(0)
            + 1;

        versions.insert(
            lorebook.current_version_id,
            StoredLorebookVersion {
                lorebook_id: lorebook.id,
                version_num: next_num,
                book_version: extract_book_version(&lorebook.config_yaml),
                snapshot: lorebook.clone(),
                created_at: Utc::now(),
            },
        );

        books.insert(lorebook.id, lorebook.clone());
        Ok(lorebook.current_version_id)
    }

    async fn delete_lorebook(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.lorebooks.write().await.remove(&id).is_some();
        if removed {
            self.lorebook_versions
                .write()
                .await
                .retain(|_, v| v.lorebook_id != id);
            self.lorebook_sessions
                .write()
                .await
                .retain(|_, s| s.lorebook_id != id);
        }
        Ok(removed)
    }

    async fn list_lorebook_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<LorebookVersionInfo>, StorageError> {
        let versions = self.lorebook_versions.read().await;
        let mut out: Vec<LorebookVersionInfo> = versions
            .values()
            .filter(|v| v.lorebook_id == id)
            .map(|v| LorebookVersionInfo {
                id: v.snapshot.current_version_id,
                version_num: v.version_num,
                book_version: v.book_version.clone(),
                created_at: v.created_at,
            })
            .collect();
        out.sort_by(|a, b| b.version_num.cmp(&a.version_num)); // newest first
        Ok(out)
    }

    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        // Scope the read lock so it drops before save_lorebook takes write locks.
        let snapshot = {
            let versions = self.lorebook_versions.read().await;
            match versions.get(&version_id) {
                Some(v) if v.lorebook_id == id => v.snapshot.clone(),
                _ => {
                    return Err(StorageError::InvalidInput(format!(
                        "lorebook {id} has no version {version_id} to restore"
                    )));
                }
            }
        };

        let mut restored = snapshot;
        restored.current_version_id = new_version_id;
        self.save_lorebook(&restored).await
    }
}

#[async_trait]
impl LorebookSessionStorage for MemoryStorage {
    async fn get_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<LorebookSession>, StorageError> {
        Ok(self
            .lorebook_sessions
            .read()
            .await
            .get(&conversation_id)
            .cloned())
    }

    async fn save_lorebook_session(&self, session: &LorebookSession) -> Result<(), StorageError> {
        self.lorebook_sessions
            .write()
            .await
            .insert(session.conversation_id, session.clone());
        Ok(())
    }

    async fn delete_lorebook_session(
        &self,
        conversation_id: Snowflake,
    ) -> Result<bool, StorageError> {
        Ok(self
            .lorebook_sessions
            .write()
            .await
            .remove(&conversation_id)
            .is_some())
    }
}
