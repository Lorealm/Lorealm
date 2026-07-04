use async_trait::async_trait;

use crate::{StorageError, models::{AgentGrant, AgentObservation, Realm, RealmAgent, Snowflake, TickCommit}};

#[async_trait]
pub trait RealmStorage {
    // on the Storage trait
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError>;
    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError>;
    
    async fn list_realm_agents(&self, realm_id: Snowflake) -> Result<Vec<RealmAgent>, StorageError>;
    async fn create_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError>;
    async fn list_agent_grants(&self, agent_id: Snowflake) -> Result<Vec<AgentGrant>, StorageError>;
    async fn set_agent_grants(&self, agent_id: Snowflake, grants: &[AgentGrant]) -> Result<(), StorageError>;
    
    /// The agent's log, oldest-first. `before` pages backward for long logs.
    async fn list_observations(
        &self,
        agent_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError>;
    
    /// Atomic: insert tick row + events + observations, UPDATE realms
    /// SET state = new_state, tick = tick + 1
    /// WHERE id = realm_id AND tick = expected_tick.
    /// Zero rows updated => Err(StorageError::TickConflict) and full rollback.
    async fn commit_tick(&self, commit: TickCommit) -> Result<(), StorageError>;
    
    // rewind/fork support — not needed for Milestone 1, listed so the trait
    // shape is visible; implement when fork lands:
    // async fn nearest_snapshot(&self, realm_id, at_or_before: i64) -> ...
    // async fn list_ticks_range(&self, realm_id, from: i64, to: i64) -> ...
}