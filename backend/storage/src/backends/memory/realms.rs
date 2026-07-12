use async_trait::async_trait;

use crate::{
    RealmStorage, StorageError,
    backends::MemoryStorage,
    models::{
        AgentGrant, AgentObservation, AgentToolBinding, Realm, RealmAgent, RealmEvent,
        RealmSummary, RealmTick, Snowflake, TickCommit,
    },
};

#[async_trait]
impl RealmStorage for MemoryStorage {
    async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, StorageError> {
        let summaries = self
            .realms
            .read()
            .await
            .values()
            .filter(|r| r.owner_id == user_id)
            .map(|r| RealmSummary {
                id: r.id,
                owner_id: r.owner_id,
                name: r.name.clone(),
                status: r.status.clone(),
            })
            .collect();

        Ok(summaries)
    }
    async fn get_realm(&self, id: Snowflake) -> Result<Option<Realm>, StorageError> {
        Ok(self.realms.read().await.get(&id).cloned())
    }
    async fn create_realm(&self, realm: &Realm) -> Result<(), StorageError> {
        self.realms.write().await.insert(realm.id, realm.clone());

        Ok(())
    }

    async fn delete_realm(&self, id: Snowflake) -> Result<(), StorageError> {
        self.realms.write().await.remove(&id);

        // cascade, mirroring the FK behavior of the SQL backends
        let agent_ids: Vec<Snowflake> = {
            let mut agents = self.realm_agents.write().await;
            let ids: Vec<Snowflake> = agents
                .values()
                .filter(|a| a.realm_id == id)
                .map(|a| a.id)
                .collect();
            agents.retain(|_, a| a.realm_id != id);
            ids
        };
        {
            let mut grants = self.agent_grants.write().await;
            let mut tools = self.agent_tools.write().await;
            for agent_id in &agent_ids {
                grants.remove(agent_id);
                tools.remove(agent_id);
            }
        }
        self.realm_ticks.write().await.retain(|t| t.realm_id != id);
        self.realm_events
            .write()
            .await
            .retain(|_, e| e.realm_id != id);
        self.agent_observations
            .write()
            .await
            .retain(|_, o| !agent_ids.contains(&o.agent_id));

        Ok(())
    }

    async fn list_realm_agents(
        &self,
        realm_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, StorageError> {
        let realm_agents = self.realm_agents.read();
        let realms = self.realms.read().await;
        if !realms.contains_key(&realm_id) {
            return Err(StorageError::NotFound(format!("realm: {realm_id}")));
        }

        let mut agents: Vec<RealmAgent> = realm_agents
            .await
            .values()
            .filter(|a| a.realm_id == realm_id)
            .cloned()
            .collect();
        agents.sort_by_key(|a| (a.turn_order, a.id));

        Ok(agents)
    }

    async fn get_realm_agent(&self, agent_id: Snowflake) -> Result<Option<RealmAgent>, StorageError> {
        Ok(self.realm_agents.read().await.get(&agent_id).cloned())
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
        // no rows is a valid state, not an error — mirror the SQL backends
        let grants = self.agent_grants.read().await;

        Ok(grants.get(&agent_id).cloned().unwrap_or_default())
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

    async fn list_agent_tools(
        &self,
        agent_id: Snowflake,
    ) -> Result<Vec<AgentToolBinding>, StorageError> {
        let mut bindings = self
            .agent_tools
            .read()
            .await
            .get(&agent_id)
            .cloned()
            .unwrap_or_default();
        bindings.sort_by_key(|b| b.tool_id);
        Ok(bindings)
    }

    async fn set_agent_tools(
        &self,
        agent_id: Snowflake,
        bindings: &[AgentToolBinding],
    ) -> Result<(), StorageError> {
        self.agent_tools
            .write()
            .await
            .insert(agent_id, bindings.to_vec());
        Ok(())
    }

    async fn list_realm_events(
        &self,
        realm_id: Snowflake,
        before_tick: Option<i64>,
        limit: i64,
    ) -> Result<Vec<RealmEvent>, StorageError> {
        let events = self.realm_events.read().await;
        let mut rows: Vec<RealmEvent> = events
            .values()
            .filter(|e| e.realm_id == realm_id && before_tick.is_none_or(|b| e.tick < b))
            .cloned()
            .collect();
        rows.sort_by_key(|e| (e.tick, e.seq));
        if rows.len() > limit as usize {
            rows.drain(..rows.len() - limit as usize); // keep the most recent `limit`, oldest-first
        }
        Ok(rows)
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
