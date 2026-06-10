use crate::backends::MemoryStorage;
use crate::error::StorageError;
use crate::models::{
    Character, CharacterSummary, CharacterVersionInfo, Snowflake, VersionedSaveMode,
};
use crate::storage::CharacterStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub(crate) struct StoredVersion {
    pub(crate) version_id: Snowflake,
    pub(crate) character_id: Snowflake,
    pub(crate) version_num: u32,
    pub(crate) card_version: String,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) character: Character,
}

#[async_trait]
impl CharacterStorage for MemoryStorage {
    async fn list_characters(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<CharacterSummary>, StorageError> {
        let chars = self.characters.read().await;
        let summaries = chars
            .iter()
            .filter(|c| c.1.user_id == user_id)
            .map(|o| CharacterSummary {
                id: o.1.id,
                name: o.1.definition.name.clone(),
                user_id,
                display_name: o.1.metadata.display_name.clone(),
                tags: o.1.metadata.tags.clone(),
                avatar_uri: o.1.metadata.avatar_uri.clone(),
                card_version: o.1.metadata.card_version.clone(),
                lorebook_id: o.1.lorebook_id,
                updated_at: o.1.metadata.updated_at,
            })
            .collect();

        Ok(summaries)
    }

    async fn get_character(&self, id: Snowflake) -> Result<Option<Character>, StorageError> {
        let chars = self.characters.read().await;
        Ok(chars.get(&id).cloned())
    }

    async fn get_character_at_version(
        &self,
        _id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Character>, StorageError> {
        let versions = self.character_versions.read().await;
        Ok(versions.get(&version_id).map(|v| {
            let mut c = v.character.clone();
            c.current_version_id = version_id;
            c
        }))
    }

    async fn list_characters_by_owner(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<Character>, StorageError> {
        let chars = self.characters.read().await;
        Ok(chars
            .values()
            .filter(|c| c.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn save_character(
        &self,
        character: &Character,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        match mode {
            VersionedSaveMode::NewVersion => {
                let next_version_num = {
                    let versions = self.character_versions.read().await;
                    versions
                        .values()
                        .filter(|v| v.character_id == character.id)
                        .map(|v| v.version_num)
                        .max()
                        .map(|n| n + 1)
                        .unwrap_or(1)
                };

                let stored = StoredVersion {
                    version_id: character.current_version_id,
                    character_id: character.id,
                    version_num: next_version_num,
                    card_version: character.metadata.card_version.clone(),
                    created_at: Utc::now(),
                    character: character.clone(),
                };

                let mut versions = self.character_versions.write().await;
                versions.insert(character.current_version_id, stored);
            }
            VersionedSaveMode::UpdateCurrent => {
                let mut versions = self.character_versions.write().await;
                let existing =
                    versions
                        .get_mut(&character.current_version_id)
                        .ok_or_else(|| {
                            StorageError::NotFound(format!(
                                "save_character: version '{}' not found for in-place update",
                                character.current_version_id
                            ))
                        })?;

                existing.card_version = character.metadata.card_version.clone();
                existing.character = character.clone();
            }
        }

        {
            let mut chars = self.characters.write().await;
            chars.insert(character.id, character.clone());
        }

        Ok(character.current_version_id)
    }

    async fn delete_character(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = {
            let mut chars = self.characters.write().await;
            chars.remove(&id).is_some()
        };
        {
            let mut versions = self.character_versions.write().await;
            versions.retain(|_, v| v.character_id != id);
        }
        Ok(removed)
    }

    async fn list_character_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<CharacterVersionInfo>, StorageError> {
        let versions = self.character_versions.read().await;
        let mut out: Vec<_> = versions
            .values()
            .filter(|v| v.character_id == id)
            .map(|v| CharacterVersionInfo {
                id: v.version_id,
                version_num: v.version_num,
                card_version: v.card_version.clone(),
                created_at: v.created_at,
            })
            .collect();
        out.sort_by(|a, b| b.version_num.cmp(&a.version_num));
        Ok(out)
    }

    async fn restore_character_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let Some(character) = self.get_character_at_version(id, version_id).await? else {
            return Err(StorageError::NotFound(format!(
                "character {id} version {version_id} not found"
            )));
        };
        let mut to_save = character;
        to_save.current_version_id = new_version_id;
        self.save_character(&to_save, VersionedSaveMode::NewVersion)
            .await
    }

    async fn get_latest_version(&self, id: Snowflake) -> Result<Snowflake, StorageError> {
        self.character_versions
            .read()
            .await
            .iter()
            .max_by_key(|x| x.0.0)
            .map(|x| *x.0)
            .ok_or_else(|| StorageError::NotFound(format!("found no versions for character {id}")))
    }
}
