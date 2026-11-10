use async_trait::async_trait;

use crate::{
    StorageError,
    models::{Realm, RealmCast, RealmSummary, RealmVersionInfo, Snowflake, VersionedSaveMode},
};

#[async_trait]
pub trait RealmStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError>;

    /// Load the current version of a realm.
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError>;

    /// Load a specific version of a realm from its snapshot.
    async fn get_realm_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Realm>, StorageError>;

    /// Create or update a realm. Writes the head and, per `mode`, either
    /// appends a version row or updates the current one in place. Tags are
    /// resynced.
    async fn save_realm(
        &self,
        realm: &Realm,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError>;

    /// Hard-delete a realm and all its versions/tags.
    /// Returns `true` if a row was deleted.
    async fn delete_realm(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// List version metadata for a realm, newest first.
    async fn list_realm_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, StorageError>;

    /// Roll the head back to a prior version: load that snapshot, save it
    /// as a fresh version row, and update the head. Returns the new
    /// version id.
    async fn restore_realm_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError>;

    /// The cast a conversation plays with, pinned when it starts.
    async fn get_cast(&self, conversation_id: Snowflake)
    -> Result<Option<RealmCast>, StorageError>;
    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError>;
}
