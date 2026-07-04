use async_trait::async_trait;

use crate::{
    RealmStorage, StorageError,
    backends::MemoryStorage,
    models::{AgentGrant, AgentObservation, Realm, RealmAgent, RealmTick, Snowflake, TickCommit},
};

#[async_trait]
impl RealmStorage for MemoryStorage {
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        Ok(self.realms.read().await.get(&id).cloned())
    }
    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        self.realms.write().await.insert(realm.id, realm.clone());

        Ok(())
    }

    async fn list_realm_agents(
        &self,
        realm_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, StorageError> {
        let realm_agents = self.realm_agents.read();
        let realms = self.realms.read().await;
        let Some(realm) = realms.get(&realm_id) else {
            return Err(StorageError::NotFound(format!("realm: {realm_id}")));
        };

        let agents = realm_agents
            .await
            .values()
            .filter(|a| a.id == realm.id)
            .cloned()
            .collect();

        Ok(agents)
    }
    async fn create_realm_agent(&self, agent: &RealmAgent) -> Result<(), StorageError> {
        self.realm_agents
            .write()
            .await
            .insert(agent.id, agent.clone());
        Ok(())
    }
    async fn list_agent_grants(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, StorageError> {
        let grants = self.agent_grants.read().await;

        let agent_grants = grants
            .get(&agent_id)
            .ok_or(StorageError::NotFound(format!("agent grant: {agent_id}")))?;

        Ok(agent_grants.clone())
    }
    async fn set_agent_grants(
        &self,
        agent_id: Snowflake,
        grants: &[AgentGrant],
    ) -> Result<(), StorageError> {
        let mut grant = self.agent_grants.write().await;
        grant.insert(agent_id, grants.to_vec());
        Ok(())
    }

    async fn list_observations(
        &self,
        agent_id: Snowflake,
        before: Option<Snowflake>,
        limit: i64,
    ) -> Result<Vec<AgentObservation>, StorageError> {
        let obs = self.agent_observations.read().await;
        let mut rows: Vec<AgentObservation> = obs
            .values()
            .filter(|o| o.agent_id == agent_id && before.is_none_or(|b| o.id < b))
            .cloned()
            .collect();
        rows.sort_by_key(|o| o.id);
        if rows.len() > limit as usize {
            rows.drain(..rows.len() - limit as usize); // keep the most recent `limit`, oldest-first
        }
        Ok(rows)
    }

    async fn commit_tick(&self, commit: TickCommit) -> Result<(), StorageError> {
        let mut realms = self.realms.write().await;
        let realm = realms
            .get_mut(&commit.realm_id)
            .ok_or(StorageError::NotFound(format!(
                "realm tick: {} ({})",
                commit.expected_tick, commit.realm_id
            )))?;

        if realm.tick != commit.expected_tick {
            return Err(StorageError::TickConflict(commit.realm_id));
        }

        realm.state = commit.new_state;
        realm.tick += 1;

        self.realm_ticks.write().await.push(RealmTick {
            realm_id: commit.realm_id,
            tick: commit.expected_tick,
            agent_id: commit.agent_id,
            delta: commit.delta,
            snapshot: commit.snapshot,
        });
        let mut events = self.realm_events.write().await;
        for e in commit.events {
            events.insert(e.id, e);
        }
        let mut obs = self.agent_observations.write().await;
        for o in commit.observations {
            obs.insert(o.id, o);
        }
        Ok(())
    }
}
