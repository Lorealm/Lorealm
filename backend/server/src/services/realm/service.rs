use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use lorealm_storage::{
    Storage, StorageError,
    models::{
        AgentKind, AgentObservation, AgentSpec, ChatRole, EventKind, Realm, RealmAgent, RealmEvent, RealmStatus, ScopePattern, Snowflake, StateDelta, TickCommit
    },
};
use tokio_util::sync::CancellationToken;

use crate::{
    errors::{RealmError, TargetType},
    services::{
        chat::{CancelGuard, ChatService},
        context::PromptTurn,
        realm::TurnPolicy,
        snowflake::SnowflakeService,
        state::{ReconstructedState, StatePool},
    },
};

const MAIN_SCOPE: &str = "main";

const SNAPSHOT_INTERVAL: i64 = 50;
const OBSERVATION_WINDOW: i64 = 200;

pub struct RealmService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    chat: Arc<ChatService>,
    policy: Arc<dyn TurnPolicy>,
    cancellations: Arc<DashMap<Snowflake, CancellationToken>>,
}

pub enum TickResult {
    /// An LLM agent acted and the tick committed.
    Committed { tick: i64, events: Vec<RealmEvent> },
    /// The policy selected a player; nothing committed. The client should
    /// prompt that user and call submit_player_turn.
    AwaitingPlayer { agent_id: Snowflake },
}

impl RealmService {
    pub async fn run_tick(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<TickResult, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        if !matches!(realm.status, RealmStatus::Active) {
            return Err(RealmError::NotActive(realm_id));
        }

        let mut agents = self.storage.list_realm_agents(realm_id).await?;
        agents.sort_by_key(|a| (a.turn_order, a.id));

        let agent = self
            .policy
            .next(&realm, &agents)
            .ok_or(RealmError::NoAgents(realm_id))?;

        if matches!(agent.kind, AgentKind::Player) {
            return Ok(TickResult::AwaitingPlayer { agent_id: agent.id });
        }

        // --- project: the agent's slice of the world ---
        let world: StatePool =
            serde_json::from_value(realm.state.clone()).map_err(RealmError::StateCorrupt)?;
        let grants: Vec<ScopePattern> = self
            .storage
            .list_agent_grants(agent.id)
            .await?
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let slice: StatePool = world
            .iter()
            .filter(|(ns, _)| grants.iter().any(|g| g.matches(ns)))
            .map(|(ns, vars)| (ns.clone(), vars.clone()))
            .collect();

        let observations = self
            .storage
            .list_observations(agent.id, None, OBSERVATION_WINDOW)
            .await?;
        let turns: Vec<PromptTurn> = observations
            .iter()
            .map(PromptTurn::from_observation)
            .collect();

        let spec = self.spec_for(&realm, agent)?;
        let baseline = ReconstructedState {
            persistent: slice,
            activation: HashMap::new(), // per-agent activation persistence: deferred, on record
        };

        // --- act ---
        let cancel = CancellationToken::new();
        self.cancellations.insert(realm_id, cancel.clone());
        let _guard = CancelGuard {
            map: Arc::clone(&self.cancellations),
            key: realm_id,
        };

        let outcome = self
            .chat
            .run_agent_turn(&spec, &turns, &baseline, &cancel)
            .await?;

        // --- emit + fan out + commit ---
        let event = RealmEvent {
            id: self.snowflake.generate(),
            realm_id,
            tick: realm.tick,
            seq: 0,
            source: Some(agent.id),
            scope: MAIN_SCOPE.to_string(),
            kind: EventKind::Spoke,
            content: outcome.spoken.clone(),
            payload: serde_json::json!({}),
        };

        let observations = self.fan_out(&agents, &[&event], &agent.name).await?;

        let outcome_meta = serde_json::json!({
            "reasoning": outcome.reasoning,
            "tool_calls": outcome.tool_calls,
            "widgets": outcome.widgets,
        });

        self.commit(
            &realm,
            Some(agent.id),
            vec![event],
            observations,
            outcome_meta,
        )
        .await
    }

    pub async fn submit_player_turn(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        content: String,
    ) -> Result<TickResult, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        let mut agents = self.storage.list_realm_agents(realm_id).await?;
        agents.sort_by_key(|a| (a.turn_order, a.id));

        let agent = self
            .policy
            .next(&realm, &agents)
            .ok_or(RealmError::NoAgents(realm_id))?;

        // It must actually be this user's player-agent's turn.
        if !matches!(agent.kind, AgentKind::Player) || agent.user_id != Some(user_id) {
            return Err(RealmError::NotYourTurn(agent.id));
        }

        let event = RealmEvent {
            id: self.snowflake.generate(),
            realm_id,
            tick: realm.tick,
            seq: 0,
            source: Some(agent.id),
            scope: MAIN_SCOPE.to_string(),
            kind: EventKind::Spoke,
            content,
            payload: serde_json::json!({}),
        };
        let observations = self.fan_out(&agents, &[&event], &agent.name).await?;

        self.commit(
            &realm,
            Some(agent.id),
            vec![event],
            observations,
            serde_json::json!({}),
        )
        .await
    }

    async fn owned_realm(&self, realm_id: Snowflake, user_id: Snowflake) -> Result<Realm, RealmError> {
        let realm = self
            .storage
            .get_realm(realm_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Realm, realm_id))?;

        if realm.owner_id != user_id {
            return Err(RealmError::NotOwned(TargetType::Realm, realm_id))
        }

        Ok(realm)
    }
}

impl RealmService {
    async fn fan_out(
        &self,
        agents: &[RealmAgent],
        events: &[&RealmEvent],
        source_name: &str,
    ) -> Result<Vec<AgentObservation>, RealmError> {
        let mut out = Vec::new();
        for observer in agents {
            let grants: Vec<ScopePattern> = self
                .storage
                .list_agent_grants(observer.id)
                .await?
                .iter()
                .map(|g| ScopePattern::parse(&g.scope))
                .collect();

            for event in events {
                if !grants.iter().any(|g| g.matches(&event.scope)) {
                    continue;
                }
                let own = event.source == Some(observer.id);
                out.push(AgentObservation {
                    id: self.snowflake.generate(),
                    agent_id: observer.id,
                    event_id: Some(event.id),
                    tick: event.tick,
                    // Your own words render as assistant turns; everyone
                    // else's arrive as attributed user turns.
                    role: if own {
                        ChatRole::Assistant
                    } else {
                        ChatRole::User
                    },
                    content: if own {
                        event.content.clone()
                    } else {
                        format!("{source_name}: {}", event.content)
                    },
                });
            }
        }
        Ok(out)
    }

    async fn commit(
        &self,
        realm: &Realm,
        agent_id: Option<Snowflake>,
        events: Vec<RealmEvent>,
        observations: Vec<AgentObservation>,
        outcome: serde_json::Value,
    ) -> Result<TickResult, RealmError> {
        let delta = serde_json::to_value(StateDelta::default())?;
        let new_state = realm.state.clone();

        let snapshot = ((realm.tick + 1) % SNAPSHOT_INTERVAL == 0).then(|| new_state.clone());

        let events_out = events.clone();
        self.storage
            .commit_tick(TickCommit {
                realm_id: realm.id,
                expected_tick: realm.tick,
                agent_id,
                delta,
                snapshot,
                outcome,
                new_state,
                events,
                observations,
            })
            .await
            .map_err(|e| match e {
                StorageError::TickConflict(_) => RealmError::TickConflict(realm.id),
                other => other.into(),
            })?;

        Ok(TickResult::Committed {
            tick: realm.tick,
            events: events_out,
        })
    }

    fn spec_for(&self, realm: &Realm, agent: &RealmAgent) -> Result<AgentSpec, RealmError> {
        let (character_id, char_version_id, template_id, profile_id) = match (
            agent.character_id,
            agent.char_version_id,
            agent.template_id,
            agent.profile_id,
        ) {
            (Some(c), Some(cv), Some(t), Some(p)) => (c, cv, t, p),
            _ => return Err(RealmError::MalformedAgent(agent.id)),
        };
        Ok(AgentSpec {
            owner: realm.owner_id,
            character_id,
            char_version_id,
            template_id,
            persona_id: agent
                .persona_id
                .ok_or(RealmError::MalformedAgent(agent.id))?, // TODO: make personas optional
            tokenizer_id: realm.tokenizer_id,
            profile_id,
        })
    }
}
