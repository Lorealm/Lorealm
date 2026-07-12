use std::{
    collections::HashMap,
    sync::{Arc, Mutex as StdMutex},
};

use chrono::Utc;
use dashmap::DashMap;
use lorealm_providers::Tool;
use lorealm_storage::{
    Storage, StorageError,
    models::{
        AgentAccess, AgentGrant, AgentKind, AgentObservation, AgentSpec, AgentToolBinding,
        ChatRole, EventKind, Realm, RealmAgent, RealmEvent, RealmStatus, RealmSummary,
        ScopePattern, Snowflake, StateDelta, TickCommit, UserTool,
    },
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::{RealmError, TargetType},
    routes::{CreateAgentRequest, PlayerTurnRequest},
    services::{
        character::CharacterService,
        chat::{AgentTurn, CancelGuard, ChatService},
        connection::ConnectionService,
        context::{ContextService, PromptTurn},
        persona::PersonaService,
        realm::{ActionSpec, RoundRobin, TickResult, TurnPolicy},
        snowflake::SnowflakeService,
        state::{ReconstructedState, StatePool, StateService, apply_delta},
        tools::{CompiledUserTool, ToolPolicy, ToolSignature, build_tool_registry},
    },
};

// TODO: consider moving out to a per-realm/per-agent config object
const MAIN_SCOPE: &str = "main";
const SNAPSHOT_INTERVAL: i64 = 50;
const OBSERVATION_WINDOW: i64 = 200;
const EVENT_WINDOW: i64 = 500;

pub struct RealmService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    chat: Arc<ChatService>,
    characters: Arc<CharacterService>,
    templates: Arc<ContextService>,
    connection: Arc<ConnectionService>,
    personas: Arc<PersonaService>,
    state: Arc<StateService>,
    policy: Arc<dyn TurnPolicy>,
    cancellations: Arc<DashMap<Snowflake, CancellationToken>>,
}

impl RealmService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        chat: Arc<ChatService>,
        characters: Arc<CharacterService>,
        templates: Arc<ContextService>,
        connection: Arc<ConnectionService>,
        personas: Arc<PersonaService>,
        state: Arc<StateService>,
    ) -> Self {
        Self {
            storage,
            snowflake,
            chat,
            characters,
            templates,
            connection,
            personas,
            state,
            policy: Arc::new(RoundRobin),
            cancellations: Arc::new(DashMap::new()),
        }
    }

    pub async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, RealmError> {
        let realms = self.storage.list_realms(user_id).await?;

        Ok(realms)
    }

    pub async fn get_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;

        Ok(realm)
    }

    pub async fn list_agents(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        let agents = self.storage.list_realm_agents(realm_id).await?;

        Ok(agents)
    }

    pub async fn list_agent_grants(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, RealmError> {
        self.owned_agent(realm_id, agent_id, user_id).await?;
        let grants = self.storage.list_agent_grants(agent_id).await?;

        Ok(grants)
    }

    pub async fn list_observations(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        before: Option<Snowflake>,
    ) -> Result<Vec<AgentObservation>, RealmError> {
        self.viewable_agent(realm_id, agent_id, user_id).await?;
        let observations = self
            .storage
            .list_observations(agent_id, before, OBSERVATION_WINDOW)
            .await?;

        Ok(observations)
    }

    /// The agent's action space
    pub async fn list_actions(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let (_realm, agent) = self.viewable_agent(realm_id, agent_id, user_id).await?;

        let mut out = Vec::new();
        for binding in self.storage.list_agent_tools(agent.id).await? {
            let Some(tool) = self.storage.get_tool(binding.tool_id).await? else {
                continue;
            };
            out.push(ActionSpec {
                tool_id: tool.id,
                name: tool.name,
                display_name: tool.display_name,
                description: tool.description,
                params: tool.params,
                event_scope: binding.event_scope,
            });
        }

        Ok(out)
    }

    /// The agent's grant-filtered view of the world state.
    pub async fn agent_state_slice(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<StatePool, RealmError> {
        let (realm, agent) = self.viewable_agent(realm_id, agent_id, user_id).await?;

        let world: StatePool =
            serde_json::from_value(realm.state.clone()).map_err(RealmError::StateCorrupt)?;
        let read_scopes: Vec<ScopePattern> = self
            .storage
            .list_agent_grants(agent.id)
            .await?
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();

        Ok(world
            .into_iter()
            .filter(|(ns, _)| read_scopes.iter().any(|g| g.matches(ns)))
            .collect())
    }

    pub async fn list_events(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        before_tick: Option<i64>,
    ) -> Result<Vec<RealmEvent>, RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        let events = self
            .storage
            .list_realm_events(realm_id, before_tick, EVENT_WINDOW)
            .await?;

        Ok(events)
    }

    pub async fn create_realm(
        &self,
        name: impl Into<String>,
        tok_id: Option<Snowflake>,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let tokenizer_id = if let Some(id) = tok_id {
            id
        } else {
            self.state
                .get_active_tokenizer_id(user_id)
                .await?
                .ok_or(RealmError::Create(
                    "could not create a realm due to missing tokenizers".to_string(),
                ))?
        };
        let realm = Realm {
            id: self.snowflake.generate(),
            owner_id: user_id,
            name: name.into(),
            tick: 0,
            state: json!({}),
            tokenizer_id,
            status: RealmStatus::Active,
        };

        self.storage.create_realm(&realm).await?;

        Ok(realm)
    }

    pub async fn delete_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        if let Some(cancel_token) = self.cancellations.get(&realm_id) {
            cancel_token.cancel();
        }
        self.storage.delete_realm(realm_id).await?;

        Ok(())
    }

    pub async fn create_agent(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let _realm = self.owned_realm(realm_id, user_id).await?;

        let (character_id, char_version_id, template_id, profile_id, agent_user_id) =
            match req.kind {
                AgentKind::Player => {
                    if req.character_id.is_some()
                        || req.char_version_id.is_some()
                        || req.template_id.is_some()
                        || req.profile_id.is_some()
                    {
                        return Err(RealmError::InvalidAgent(
                            "player agents cannot have character/template/profile".into(),
                        ));
                    }
                    (None, None, None, None, Some(user_id))
                }
                AgentKind::Llm => {
                    // both-or-neither, validated for ownership when present
                    let character = match (req.character_id, req.char_version_id) {
                        (Some(c), Some(v)) => {
                            self.characters.load_version(c, v, user_id).await?;
                            (Some(c), Some(v))
                        }
                        (None, None) => (None, None),
                        _ => {
                            return Err(RealmError::InvalidAgent(
                                "character_id and char_version_id must be set together".into(),
                            ));
                        }
                    };

                    let template_id =
                        match req.template_id {
                            Some(id) => self.templates.get_template(id, user_id).await?.id,
                            None => self.state.get_active_template_id(user_id).await?.ok_or(
                                RealmError::NotFound(TargetType::ContextTemplate, user_id),
                            )?,
                        };
                    let profile_id = match req.profile_id {
                        Some(id) => self.connection.get_profile(user_id, id).await?.id,
                        None => self.state.get_active_profile(user_id).await?.id,
                    };

                    (
                        character.0,
                        character.1,
                        Some(template_id),
                        Some(profile_id),
                        None,
                    )
                }
            };

        if let Some(id) = req.persona_id {
            self.personas.get_persona(id, user_id).await?;
        }

        for binding in &req.tools {
            let tool = self
                .storage
                .get_tool(binding.tool_id)
                .await?
                .ok_or(RealmError::NotFound(TargetType::UserTool, binding.tool_id))?;
            if tool.user_id != user_id {
                return Err(RealmError::NotOwned(TargetType::UserTool, binding.tool_id));
            }
        }

        let turn_order = self.storage.list_realm_agents(realm_id).await?.len() as i32;

        let agent = RealmAgent {
            id: self.snowflake.generate(),
            realm_id,
            name: req.name,
            kind: req.kind,
            character_id,
            char_version_id,
            template_id,
            profile_id,
            user_id: agent_user_id,
            persona_id: req.persona_id,
            turn_order,
            extensions: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.create_realm_agent(&agent).await?;

        if !req.grants.is_empty() {
            let grants: Vec<AgentGrant> = req
                .grants
                .into_iter()
                .map(|g| AgentGrant {
                    agent_id: agent.id,
                    scope: g.scope,
                    access: g.access,
                })
                .collect();
            self.storage.set_agent_grants(agent.id, &grants).await?;
        }

        if !req.tools.is_empty() {
            let bindings: Vec<AgentToolBinding> = req
                .tools
                .iter()
                .map(|t| AgentToolBinding {
                    tool_id: t.tool_id,
                    event_scope: t.event_scope.clone(),
                })
                .collect();
            self.storage.set_agent_tools(agent.id, &bindings).await?;
        }

        Ok(agent)
    }

    pub async fn set_grants(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        grants: Vec<AgentGrant>,
    ) -> Result<(), RealmError> {
        self.owned_agent(realm_id, agent_id, user_id).await?;
        self.storage.set_agent_grants(agent_id, &grants).await?;

        Ok(())
    }

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

        let world: StatePool =
            serde_json::from_value(realm.state.clone()).map_err(RealmError::StateCorrupt)?;
        let grants = self.storage.list_agent_grants(agent.id).await?;
        let read_scopes: Vec<ScopePattern> = grants
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let write_scopes: Vec<ScopePattern> = grants
            .iter()
            .filter(|g| matches!(g.access, AgentAccess::ReadWrite))
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let slice: StatePool = world
            .iter()
            .filter(|(ns, _)| read_scopes.iter().any(|g| g.matches(ns)))
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
            activation: HashMap::new(),
        };
        let bindings = self.storage.list_agent_tools(agent.id).await?;
        let policy = ToolPolicy {
            write_scopes: Some(Arc::new(write_scopes)),
            tool_ids: Some(bindings.iter().map(|b| b.tool_id).collect()),
        };

        let cancel = CancellationToken::new();
        self.cancellations.insert(realm_id, cancel.clone());
        let _guard = CancelGuard {
            map: Arc::clone(&self.cancellations),
            key: realm_id,
        };

        let AgentTurn { outcome, delta } = self
            .chat
            .run_agent_turn(&spec, &turns, &baseline, &policy, &cancel)
            .await?;

        let mut new_world = world;
        apply_delta(&mut new_world, &delta);

        let action_scopes = self.action_scopes(&bindings).await?;

        let mut events = Vec::new();
        if !outcome.spoken.trim().is_empty() {
            events.push(self.event(
                &realm,
                agent.id,
                events.len(),
                MAIN_SCOPE.to_string(),
                EventKind::Spoke,
                outcome.spoken.clone(),
                json!({}),
            ));
        }

        let mut actor_notes = Vec::new();
        for call in &outcome.tool_calls {
            let Some(scope) = action_scopes.get(&call.name) else {
                continue;
            };
            if call.is_error {
                continue;
            }
            events.push(self.event(
                &realm,
                agent.id,
                events.len(),
                scope.clone().unwrap_or_default(),
                EventKind::Acted,
                // TODO: remove hard-coded formatting
                format!("[{} used {}]", agent.name, call.name),
                json!({ "tool": call.name, "input": call.input }),
            ));


            // TODO: remove hard-coded formatting
            actor_notes.push(format!("[{} result: {}]", call.name, call.output));
        }

        self.push_state_events(&realm, agent.id, &delta, &mut events);

        let mut observations = self.fan_out(&agents, &events, &agent.name).await?;
        observations.extend(self.actor_notes(agent.id, realm.tick, actor_notes));

        let outcome_meta = serde_json::json!({
            "reasoning": outcome.reasoning,
            "tool_calls": outcome.tool_calls,
            "widgets": outcome.widgets,
        });

        self.commit(
            &realm,
            Some(agent.id),
            events,
            observations,
            outcome_meta,
            delta,
            serde_json::to_value(new_world)?,
        )
        .await
    }

    pub async fn cancel_tick(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        if let Some(cancel_token) = self.cancellations.get(&realm_id) {
            cancel_token.cancel();
        }

        Ok(())
    }

    pub async fn submit_player_turn(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: PlayerTurnRequest,
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

        if !matches!(agent.kind, AgentKind::Player) || agent.user_id != Some(user_id) {
            return Err(RealmError::NotYourTurn(agent.id));
        }

        let content = req.content.unwrap_or_default();
        if content.trim().is_empty() && req.action.is_none() {
            return Err(RealmError::InvalidTurn(
                "a player turn needs text, an action, or both".into(),
            ));
        }

        let world: StatePool =
            serde_json::from_value(realm.state.clone()).map_err(RealmError::StateCorrupt)?;

        let mut events = Vec::new();
        if !content.trim().is_empty() {
            events.push(self.event(
                &realm,
                agent.id,
                events.len(),
                MAIN_SCOPE.to_string(),
                EventKind::Spoke,
                content,
                json!({}),
            ));
        }

        // A player action runs the exact same weaver pipeline
        // an LLM agent's tool call goes through
        let mut delta = StateDelta::default();
        let mut actor_notes = Vec::new();
        let mut outcome_meta = json!({});
        if let Some(action) = req.action {
            let (tool, binding) = self.bound_tool(agent.id, action.tool_id).await?;
            let grants = self.storage.list_agent_grants(agent.id).await?;
            let (output, d) = self
                .execute_action(&tool, &grants, action.args.clone(), &world)
                .await?;

            events.push(self.event(
                &realm,
                agent.id,
                events.len(),
                binding.event_scope.unwrap_or_default(),
                EventKind::Acted,
                // TODO: remove hard-coded formatting
                format!("[{} used {}]", agent.name, tool.name),
                json!({ "tool": tool.name, "input": action.args }),
            ));
            self.push_state_events(&realm, agent.id, &d, &mut events);
            // TODO: remove hard-coded formatting
            actor_notes.push(format!("[{} result: {}]", tool.name, output));
            outcome_meta =
                json!({ "action": { "tool": tool.name, "input": action.args, "output": output } });
            delta = d;
        }

        let mut new_world = world;
        apply_delta(&mut new_world, &delta);

        let mut observations = self.fan_out(&agents, &events, &agent.name).await?;
        observations.extend(self.actor_notes(agent.id, realm.tick, actor_notes));

        self.commit(
            &realm,
            Some(agent.id),
            events,
            observations,
            outcome_meta,
            delta,
            serde_json::to_value(new_world)?,
        )
        .await
    }

    async fn owned_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let realm = self
            .storage
            .get_realm(realm_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Realm, realm_id))?;

        if realm.owner_id != user_id {
            return Err(RealmError::NotOwned(TargetType::Realm, realm_id));
        }

        Ok(realm)
    }

    async fn viewable_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(Realm, RealmAgent), RealmError> {
        let realm = self
            .storage
            .get_realm(realm_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Realm, realm_id))?;
        let agent = self
            .storage
            .get_realm_agent(agent_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))?;
        if agent.realm_id != realm_id {
            return Err(RealmError::NotFound(TargetType::RealmAgent, agent_id));
        }
        if realm.owner_id != user_id && agent.user_id != Some(user_id) {
            return Err(RealmError::NotOwned(TargetType::RealmAgent, agent_id));
        }

        Ok((realm, agent))
    }

    async fn owned_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmAgent, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        let agent = self
            .storage
            .get_realm_agent(agent_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))?;
        if agent.realm_id != realm_id {
            return Err(RealmError::NotFound(TargetType::RealmAgent, agent_id));
        }

        Ok(agent)
    }
}

impl RealmService {
    async fn fan_out(
        &self,
        agents: &[RealmAgent],
        events: &[RealmEvent],
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
                let own = event.source == Some(observer.id);
                if !own && !grants.iter().any(|g| g.matches(&event.scope)) {
                    continue;
                }
                let (role, content) = match event.kind {
                    EventKind::Spoke if own => (ChatRole::Assistant, event.content.clone()),
                    EventKind::Spoke => {
                        // TODO: remove hard-coded formatting
                        (ChatRole::User, format!("{source_name}: {}", event.content))
                    }
                    _ => (ChatRole::User, event.content.clone()),
                };
                out.push(AgentObservation {
                    id: self.snowflake.generate(),
                    agent_id: observer.id,
                    event_id: Some(event.id),
                    tick: event.tick,
                    role,
                    content,
                });
            }
        }
        Ok(out)
    }

    fn event(
        &self,
        realm: &Realm,
        source: Snowflake,
        seq: usize,
        scope: String,
        kind: EventKind,
        content: String,
        payload: serde_json::Value,
    ) -> RealmEvent {
        RealmEvent {
            id: self.snowflake.generate(),
            realm_id: realm.id,
            tick: realm.tick,
            seq: seq as i32,
            source: Some(source),
            scope,
            kind,
            content,
            payload,
        }
    }

    fn push_state_events(
        &self,
        realm: &Realm,
        source: Snowflake,
        delta: &StateDelta,
        events: &mut Vec<RealmEvent>,
    ) {
        let mut namespaces: Vec<&String> =
            delta.writes.keys().chain(delta.deletes.keys()).collect();
        namespaces.sort();
        namespaces.dedup();

        for ns in namespaces {
            let mut parts = Vec::new();
            if let Some(vars) = delta.writes.get(ns) {
                let mut keys: Vec<&String> = vars.keys().collect();
                keys.sort();
                for k in keys {
                    // TODO: remove hard-coded formatting
                    parts.push(format!("{k} = {}", vars[k]));
                }
            }
            if let Some(names) = delta.deletes.get(ns) {
                for n in names {
                    parts.push(format!("-{n}"));
                }
            }
            events.push(self.event(
                realm,
                source,
                events.len(),
                ns.clone(),
                EventKind::StateChanged,
                // TODO: remove hard-coded formatting
                format!("[state {ns} changed: {}]", parts.join(", ")),
                json!({ "writes": delta.writes.get(ns), "deletes": delta.deletes.get(ns) }),
            ));
        }
    }

    async fn action_scopes(
        &self,
        bindings: &[AgentToolBinding],
    ) -> Result<HashMap<String, Option<String>>, RealmError> {
        let mut map = HashMap::new();
        for b in bindings {
            if let Some(tool) = self.storage.get_tool(b.tool_id).await? {
                map.insert(tool.name, b.event_scope.clone());
            }
        }
        Ok(map)
    }

    fn actor_notes(
        &self,
        agent_id: Snowflake,
        tick: i64,
        notes: Vec<String>,
    ) -> Vec<AgentObservation> {
        notes
            .into_iter()
            .map(|content| AgentObservation {
                id: self.snowflake.generate(),
                agent_id,
                event_id: None,
                tick,
                role: ChatRole::User,
                content,
            })
            .collect()
    }

    async fn bound_tool(
        &self,
        agent_id: Snowflake,
        tool_id: Snowflake,
    ) -> Result<(UserTool, AgentToolBinding), RealmError> {
        let binding = self
            .storage
            .list_agent_tools(agent_id)
            .await?
            .into_iter()
            .find(|b| b.tool_id == tool_id)
            .ok_or(RealmError::ToolNotBound(tool_id))?;
        let tool = self
            .storage
            .get_tool(tool_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::UserTool, tool_id))?;
        Ok((tool, binding))
    }

    async fn execute_action(
        &self,
        tool: &UserTool,
        grants: &[AgentGrant],
        args: serde_json::Value,
        world: &StatePool,
    ) -> Result<(serde_json::Value, StateDelta), RealmError> {
        let read_scopes: Vec<ScopePattern> = grants
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let write_scopes: Vec<ScopePattern> = grants
            .iter()
            .filter(|g| matches!(g.access, AgentAccess::ReadWrite))
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let slice: StatePool = world
            .iter()
            .filter(|(ns, _)| read_scopes.iter().any(|g| g.matches(ns)))
            .map(|(ns, vars)| (ns.clone(), vars.clone()))
            .collect();

        let pool = Arc::new(StdMutex::new(slice.clone()));
        let compiled = CompiledUserTool::build(
            ToolSignature::from(tool.clone()),
            Arc::new(build_tool_registry()),
            Arc::clone(&pool),
            Some(Arc::new(write_scopes)),
        )
        .map_err(|e| RealmError::ActionFailed(e.to_string()))?;

        let output = compiled
            .call(args)
            .await
            .map_err(|e| RealmError::ActionFailed(e.to_string()))?;

        let delta = {
            let after = pool.lock().unwrap();
            self.state.diff(&slice, &after)
        };
        Ok((output, delta))
    }

    #[allow(clippy::too_many_arguments)]
    async fn commit(
        &self,
        realm: &Realm,
        agent_id: Option<Snowflake>,
        events: Vec<RealmEvent>,
        observations: Vec<AgentObservation>,
        outcome: serde_json::Value,
        delta: StateDelta,
        new_state: serde_json::Value,
    ) -> Result<TickResult, RealmError> {
        let delta = serde_json::to_value(delta)?;

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
        let template_id = agent.template_id.ok_or(RealmError::MalformedAgent(
            agent.id,
            "llm agent has no template".to_string(),
        ))?;
        let profile_id = agent.profile_id.ok_or(RealmError::MalformedAgent(
            agent.id,
            "llm agent has no connection profile".to_string(),
        ))?;
        let character = match (agent.character_id, agent.char_version_id) {
            (Some(c), Some(v)) => Some((c, v)),
            (None, None) => None,
            _ => {
                return Err(RealmError::MalformedAgent(
                    agent.id,
                    "character_id and char_version_id must be set together".to_string(),
                ));
            }
        };
        Ok(AgentSpec {
            owner: realm.owner_id,
            character,
            template_id,
            persona_id: agent.persona_id,
            tokenizer_id: realm.tokenizer_id,
            profile_id,
        })
    }
}
