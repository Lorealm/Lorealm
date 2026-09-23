use async_trait::async_trait;

use crate::{
    StorageError,
    models::{
        AgentObservation, Realm, RealmCast, RealmEvent, RealmSummary, RealmVersionInfo, Snowflake,
        TurnCommit, VersionedSaveMode,
    },
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

    // ----- playthroughs (conversation-owned) -----

    /// The cast snapshot pinned when the conversation instantiated its realm.
    async fn get_cast(&self, conversation_id: Snowflake)
    -> Result<Option<RealmCast>, StorageError>;
    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError>;

    /// The playthrough's timeline, oldest-first. `before` pages backward by
    /// event id.
    async fn list_realm_events(
        &self,
        conversation_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<RealmEvent>, StorageError>;

    /// The agent's feed within one playthrough, oldest-first. `before`
    /// pages backward by observation id.
    async fn list_observations(
        &self,
        conversation_id: Snowflake,
        agent_handle: &str,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError>;

    /// The agent's feed restricted to the given messages (an active path),
    /// oldest-first by observation id.
    async fn list_observations_for_messages(
        &self,
        agent_handle: &str,
        message_ids: &[Snowflake],
    ) -> Result<Vec<AgentObservation>, StorageError>;

    /// Atomic insert of one committed turn's events + observations.
    async fn commit_turn(&self, commit: TurnCommit) -> Result<(), StorageError>;
}
