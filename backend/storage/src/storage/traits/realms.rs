use async_trait::async_trait;

use crate::{
    StorageError,
    models::{
        AgentGrant, AgentObservation, AgentToolBinding, Realm, RealmAgent, RealmCast, RealmEvent,
        RealmSummary, Snowflake, TurnCommit,
    },
};

#[async_trait]
pub trait RealmStorage {
    // ----- presets -----

    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError>;
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError>;
    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError>;
    /// Full-row write of the mutable preset fields (name, tokenizer, status).
    async fn update_realm(&self, realm: &Realm) -> Result<(), StorageError>;
    async fn delete_realm(&self, id: Snowflake) -> Result<(), StorageError>;

    async fn list_realm_agents(&self, realm_id: Snowflake)
    -> Result<Vec<RealmAgent>, StorageError>;
    async fn get_realm_agent(
        &self,
        agent_id: Snowflake,
    ) -> Result<Option<RealmAgent>, StorageError>;
    async fn create_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError>;
    async fn update_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError>;
    async fn delete_realm_agent(&self, agent_id: Snowflake) -> Result<(), StorageError>;

    async fn list_agent_grants(&self, agent_id: Snowflake)
    -> Result<Vec<AgentGrant>, StorageError>;
    async fn set_agent_grants(
        &self,
        agent_id: Snowflake,
        grants: &[AgentGrant],
    ) -> Result<(), StorageError>;
    async fn list_agent_tools(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentToolBinding>, StorageError>;
    async fn set_agent_tools(
        &self,
        agent_id: Snowflake,
        bindings: &[AgentToolBinding],
    ) -> Result<(), StorageError>;

    // ----- playthroughs (conversation-owned) -----

    /// The cast snapshot pinned when the conversation instantiated its realm.
    async fn get_cast(&self, conversation_id: Snowflake)
    -> Result<Option<RealmCast>, StorageError>;
    async fn put_cast(
        &self,
        conversation_id: Snowflake,
        cast: &RealmCast,
    ) -> Result<(), StorageError>;

    /// The playthrough's objective timeline, oldest-first. `before` pages
    /// backward by event id for long histories.
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
        agent_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError>;

    /// The agent's feed restricted to the given messages (an active path),
    /// oldest-first by observation id. This is the branch-aware prompt view.
    async fn list_observations_for_messages(
        &self,
        agent_id: Snowflake,
        message_ids: &[Snowflake],
    ) -> Result<Vec<AgentObservation>, StorageError>;

    /// Atomic insert of one committed turn's events + observations. The
    /// anchoring message and its state delta are persisted by the chat
    /// pipeline before this is called.
    async fn commit_turn(&self, commit: TurnCommit) -> Result<(), StorageError>;
}
