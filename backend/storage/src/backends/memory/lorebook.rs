use async_trait::async_trait;
use chrono::Utc;

use crate::error::StorageError;
use crate::models::{
    Lorebook, LorebookMetadata, LorebookVersionInfo, Snowflake, extract_book_version,
};
use crate::storage::LorebookStorage;

use super::MemoryStorage;

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
            for c in self.characters.write().await.values_mut() {
                c.lorebook_ids.retain(|l| *l != id);
            }
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
        out.sort_by_key(|x| std::cmp::Reverse(x.version_num));
        Ok(out)
    }

    async fn restore_lorebook_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
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

    async fn list_enabled_lorebooks(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Snowflake>, StorageError> {
        let ids: Vec<Snowflake> = self
            .enabled_lorebooks
            .read()
            .await
            .iter()
            .find(|b| b.0 == &user_id)
            .ok_or(StorageError::NotFound(format!(
                "No state for user: {user_id}"
            )))?
            .1
            .to_owned();

        Ok(ids)
    }

    async fn set_enabled_lorebooks(
        &self,
        user_id: Snowflake,
        ordered: &[Snowflake],
    ) -> Result<(), StorageError> {
        self.enabled_lorebooks
            .write()
            .await
            .insert(user_id, ordered.to_vec());
        Ok(())
    }
}
