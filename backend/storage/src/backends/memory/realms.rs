use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::{
    RealmStorage, StorageError,
    backends::MemoryStorage,
    models::{Realm, RealmCast, RealmSummary, RealmVersionInfo, Snowflake, VersionedSaveMode},
};

/// Head row + the metadata postgres keeps on the realms table.
#[derive(Debug, Clone)]
pub(crate) struct StoredRealm {
    pub(crate) realm: Realm,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredRealmVersion {
    pub(crate) version_id: Snowflake,
    pub(crate) realm_id: Snowflake,
    pub(crate) version_num: u32,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) realm: Realm,
}

#[async_trait]
impl RealmStorage for MemoryStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        let realms = self.realms.read().await;
        let mut summaries: Vec<RealmSummary> = realms
            .values()
            .filter(|s| s.realm.owner_id == user_id)
            .map(|s| RealmSummary {
                id: s.realm.id,
                owner_id: s.realm.owner_id,
                name: s.realm.definition.name.clone(),
                description: s.realm.definition.description.clone(),
                tags: s.realm.tags.clone(),
                origin: s.realm.origin.clone(),
                updated_at: s.updated_at,
            })
            .collect();
        summaries.sort_by_key(|x| std::cmp::Reverse(x.updated_at));

        Ok(summaries)
    }

    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        Ok(self.realms.read().await.get(&id).map(|s| s.realm.clone()))
    }

    async fn get_realm_at_version(
        &self,
        _id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Realm>, StorageError> {
        let versions = self.realm_versions.read().await;
        Ok(versions.get(&version_id).map(|v| {
            let mut r = v.realm.clone();
            r.current_version_id = version_id;
            r
        }))
    }

    async fn save_realm(
        &self,
        realm: &Realm,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        match mode {
            VersionedSaveMode::NewVersion => {
                let next_version_num = {
                    let versions = self.realm_versions.read().await;
                    versions
                        .values()
                        .filter(|v| v.realm_id == realm.id)
                        .map(|v| v.version_num)
                        .max()
                        .map(|n| n + 1)
                        .unwrap_or(1)
                };

                let stored = StoredRealmVersion {
                    version_id: realm.current_version_id,
                    realm_id: realm.id,
                    version_num: next_version_num,
                    created_at: Utc::now(),
                    realm: realm.clone(),
                };

                let mut versions = self.realm_versions.write().await;
                versions.insert(realm.current_version_id, stored);
            }
            VersionedSaveMode::UpdateCurrent => {
                let mut versions = self.realm_versions.write().await;
                let existing = versions.get_mut(&realm.current_version_id).ok_or_else(|| {
                    StorageError::NotFound(format!(
                        "save_realm: version '{}' not found for in-place update",
                        realm.current_version_id
                    ))
                })?;

                existing.realm = realm.clone();
            }
        }

        {
            let mut realms = self.realms.write().await;
            realms.insert(
                realm.id,
                StoredRealm {
                    realm: realm.clone(),
                    updated_at: Utc::now(),
                },
            );
        }

        Ok(realm.current_version_id)
    }

    async fn delete_realm(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.realms.write().await.remove(&id).is_some();
        if removed {
            self.realm_versions
                .write()
                .await
                .retain(|_, v| v.realm_id != id);
            self.conversations
                .write()
                .await
                .values_mut()
                .filter(|c| c.realm_id == Some(id))
                .for_each(|c| c.realm_id = None);
        }
        Ok(removed)
    }

    async fn list_realm_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, StorageError> {
        let versions = self.realm_versions.read().await;
        let mut out: Vec<_> = versions
            .values()
            .filter(|v| v.realm_id == id)
            .map(|v| RealmVersionInfo {
                id: v.version_id,
                version_num: v.version_num,
                created_at: v.created_at,
            })
            .collect();
        out.sort_by_key(|x| std::cmp::Reverse(x.version_num));
        Ok(out)
    }

    async fn restore_realm_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        let Some(realm) = self.get_realm_at_version(id, version_id).await? else {
            return Err(StorageError::NotFound(format!(
                "realm {id} version {version_id} not found"
            )));
        };
        let mut to_save = realm;
        to_save.current_version_id = new_version_id;
        self.save_realm(&to_save, VersionedSaveMode::NewVersion)
            .await
    }

    async fn get_cast(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<RealmCast>, StorageError> {
        Ok(self
            .conversation_casts
            .read()
            .await
            .get(&conversation_id)
            .cloned())
    }

    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError> {
        self.conversation_casts
            .write()
            .await
            .insert(conversation_id, cast.clone());
        Ok(())
    }
}
