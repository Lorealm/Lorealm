use async_trait::async_trait;

use crate::{
    RealmStorage, StorageError,
    backends::CachedStorage,
    models::{
        AgentObservation, Realm, RealmCast, RealmEvent, RealmSummary, RealmVersionInfo, Snowflake,
        TurnCommit, VersionedSaveMode,
    },
};

#[async_trait]
impl RealmStorage for CachedStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        self.backend.list_realms(user_id).await
    }
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        self.backend.get_realm(id).await
    }
    async fn get_realm_at_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
    ) -> Result<Option<Realm>, StorageError> {
        self.backend.get_realm_at_version(id, version_id).await
    }
    async fn save_realm(
        &self,
        realm: &Realm,
        mode: VersionedSaveMode,
    ) -> Result<Snowflake, StorageError> {
        self.backend.save_realm(realm, mode).await
    }
    async fn delete_realm(&self, id: Snowflake) -> Result<bool, StorageError> {
        self.backend.delete_realm(id).await
    }
    async fn list_realm_versions(
        &self,
        id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, StorageError> {
        self.backend.list_realm_versions(id).await
    }
    async fn restore_realm_version(
        &self,
        id: Snowflake,
        version_id: Snowflake,
        new_version_id: Snowflake,
    ) -> Result<Snowflake, StorageError> {
        self.backend
            .restore_realm_version(id, version_id, new_version_id)
            .await
    }

    async fn get_cast(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Option<RealmCast>, StorageError> {
        self.backend.get_cast(conversation_id).await
    }
    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError> {
        self.backend.put_cast(conversation_id, cast).await
    }

    async fn list_realm_events(
        &self,
        conversation_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<RealmEvent>, StorageError> {
        self.backend
            .list_realm_events(conversation_id, before, limit)
            .await
    }

    async fn list_observations(
        &self,
        conversation_id: Snowflake,
        agent_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError> {
        self.backend
            .list_observations(conversation_id, agent_id, before, limit)
            .await
    }

    async fn list_observations_for_messages(
        &self,
        agent_id: Snowflake,
        message_ids: &[Snowflake],
    ) -> Result<Vec<AgentObservation>, StorageError> {
        self.backend
            .list_observations_for_messages(agent_id, message_ids)
            .await
    }

    async fn commit_turn(&self, commit: TurnCommit) -> Result<(), StorageError> {
        self.backend.commit_turn(commit).await
    }
}
