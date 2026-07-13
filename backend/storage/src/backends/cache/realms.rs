use async_trait::async_trait;

use crate::{
    RealmStorage, StorageError,
    backends::CachedStorage,
    models::{
        AgentGrant, AgentObservation, AgentToolBinding, Realm, RealmAgent, RealmCast, RealmEvent,
        RealmSummary, Snowflake, TurnCommit,
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
    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        self.backend.create_realm(realm).await
    }
    async fn update_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        self.backend.update_realm(realm).await
    }
    async fn delete_realm(&self, id: Snowflake) -> Result<(), StorageError> {
        self.backend.delete_realm(id).await
    }

    async fn list_realm_agents(
        &self,
        realm_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, StorageError> {
        self.backend.list_realm_agents(realm_id).await
    }
    async fn get_realm_agent(
        &self,
        agent_id: Snowflake,
    ) -> Result<Option<RealmAgent>, StorageError> {
        self.backend.get_realm_agent(agent_id).await
    }
    async fn create_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError> {
        self.backend.create_realm_agent(agent).await
    }
    async fn update_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError> {
        self.backend.update_realm_agent(agent).await
    }
    async fn delete_realm_agent(&self, agent_id: Snowflake) -> Result<(), StorageError> {
        self.backend.delete_realm_agent(agent_id).await
    }
    async fn list_agent_tools(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentToolBinding>, StorageError> {
        self.backend.list_agent_tools(agent_id).await
    }
    async fn set_agent_tools(
        &self,
        agent_id: Snowflake,
        bindings: &[AgentToolBinding],
    ) -> Result<(), StorageError> {
        self.backend.set_agent_tools(agent_id, bindings).await
    }
    async fn list_agent_grants(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, StorageError> {
        self.backend.list_agent_grants(agent_id).await
    }
    async fn set_agent_grants(
        &self,
        agent_id: Snowflake,
        grants: &[AgentGrant],
    ) -> Result<(), StorageError> {
        self.backend.set_agent_grants(agent_id, grants).await
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
