use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex as StdMutex},
};

use chrono::Utc;
use futures_util::{Stream, StreamExt};
use lorealm_providers::Tool;
use lorealm_storage::{
    Storage,
    models::{
        ActivationState, AgentAccess, AgentGrant, AgentKind, AgentObservation, AgentSpec,
        AgentToolBinding, CastAgent, CastGrant, ChatRole, Conversation, EventKind, Realm,
        RealmAgent, RealmCast, RealmEvent, RealmStatus, RealmSummary, ScopePattern, Snowflake,
        StateDelta, TurnCommit, UserTool,
    },
};
use serde_json::json;

use crate::{
    errors::{RealmError, TargetType},
    routes::{CreateAgentRequest, GrantDto, PlayerTurnRequest, ToolBindingDto, UpdateRealmRequest},
    services::{
        character::CharacterService,
        chat::{AgentOutcome, ChatService, FullConversation, InferEvent, TurnAccumulator},
        connection::ConnectionService,
        context::{ContextService, PromptTurn},
        conversation::ConversationService,
        persona::PersonaService,
        realm::{ActionSpec, RealmStreamEvent, RoundRobin, TurnPolicy, TurnResult},
        snowflake::SnowflakeService,
        state::{CapturedState, ReconstructedState, StatePool, StateService},
        tools::{CompiledUserTool, ToolPolicy, ToolSignature, build_tool_registry},
    },
};

// TODO: consider moving out to a per-realm/per-agent config object
const MAIN_SCOPE: &str = "main";
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
    conversations: Arc<ConversationService>,
    policy: Arc<dyn TurnPolicy>,
}

impl RealmService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        chat: Arc<ChatService>,
        characters: Arc<CharacterService>,
        templates: Arc<ContextService>,
        connection: Arc<ConnectionService>,
        personas: Arc<PersonaService>,
        state: Arc<StateService>,
        conversations: Arc<ConversationService>,
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
            conversations,
            policy: Arc::new(RoundRobin),
        }
    }

    // ------------------------------------------------------------ presets

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
            tokenizer_id,
            status: RealmStatus::Draft,
        };

        self.storage.create_realm(&realm).await?;

        Ok(realm)
    }

    pub async fn update_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: UpdateRealmRequest,
    ) -> Result<Realm, RealmError> {
        let mut realm = self.owned_realm(realm_id, user_id).await?;

        if let Some(name) = req.name {
            realm.name = name;
        }
        if let Some(status) = req.status {
            realm.status = status;
        }
        if let Some(tokenizer_id) = req.tokenizer_id {
            realm.tokenizer_id = tokenizer_id;
        }

        self.storage.update_realm(&realm).await?;

        Ok(realm)
    }

    pub async fn delete_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        self.storage.delete_realm(realm_id).await?;

        Ok(())
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

    /// The agent's action space as currently authored in the preset.
    pub async fn list_actions(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let agent = self.owned_agent(realm_id, agent_id, user_id).await?;

        let bindings = self.storage.list_agent_tools(agent.id).await?;
        self.action_specs(&bindings).await
    }

    pub async fn create_agent(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        let parts = self.validate_agent(user_id, &req).await?;
        let turn_order = self.storage.list_realm_agents(realm_id).await?.len() as i32;

        let agent = RealmAgent {
            id: self.snowflake.generate(),
            realm_id,
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            turn_order,
            extensions: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.create_realm_agent(&agent).await?;
        self.apply_agent_bindings(agent.id, req.grants, req.tools)
            .await?;

        Ok(agent)
    }

    /// Full-replace edit of a preset agent (grants and tools included).
    /// In-flight playthroughs keep their snapshot; only new ones see this.
    pub async fn update_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let existing = self.owned_agent(realm_id, agent_id, user_id).await?;

        let parts = self.validate_agent(user_id, &req).await?;

        let agent = RealmAgent {
            id: existing.id,
            realm_id,
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            turn_order: existing.turn_order,
            extensions: existing.extensions,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        self.storage.update_realm_agent(&agent).await?;
        self.apply_agent_bindings(agent.id, req.grants, req.tools)
            .await?;

        Ok(agent)
    }

    pub async fn delete_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_agent(realm_id, agent_id, user_id).await?;
        self.storage.delete_realm_agent(agent_id).await?;

        Ok(())
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

    // -------------------------------------------------------- playthroughs

    /// Instantiate a realm: create a conversation bound to it and pin the
    /// cast snapshot. No seed state is copied — there is none; the opening
    /// pool is emergent, written by the weaver expressions the realm's
    /// lorebooks/tools run over the first turns.
    pub async fn start_conversation(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<FullConversation, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        if matches!(realm.status, RealmStatus::Archived) {
            return Err(RealmError::NotPlayable(realm_id));
        }

        let agents = self.storage.list_realm_agents(realm_id).await?;
        if agents.is_empty() {
            return Err(RealmError::NoAgents(realm_id));
        }

        let mut cast_agents = Vec::with_capacity(agents.len());
        for a in &agents {
            let grants = self
                .storage
                .list_agent_grants(a.id)
                .await?
                .into_iter()
                .map(|g| CastGrant {
                    scope: g.scope,
                    access: g.access,
                })
                .collect();
            let tools = self.storage.list_agent_tools(a.id).await?;
            cast_agents.push(CastAgent {
                id: a.id,
                name: a.name.clone(),
                kind: a.kind.clone(),
                character_id: a.character_id,
                char_version_id: a.char_version_id,
                template_id: a.template_id,
                profile_id: a.profile_id,
                user_id: a.user_id,
                persona_id: a.persona_id,
                turn_order: a.turn_order,
                grants,
                tools,
            });
        }
        // list_realm_agents returns (turn_order, id) order — the snapshot
        // preserves it; the round-robin index runs over this vec.

        let convo = Conversation {
            id: self.snowflake.generate(),
            user_id,
            character_id: None,
            char_version_id: None,
            realm_id: Some(realm_id),
            active_leaf_id: None,
            title: Some(realm.name.clone()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.storage.create_conversation(&convo).await?;
        self.storage
            .put_cast(
                convo.id,
                &RealmCast {
                    realm_id,
                    tokenizer_id: realm.tokenizer_id,
                    agents: cast_agents,
                },
            )
            .await?;

        Ok(FullConversation {
            metadata: convo,
            messages: Vec::new(),
        })
    }

    /// Run the next LLM turn and stream it. `target` regenerates: the new
    /// message becomes a sibling of `target`, i.e. the same agent retakes
    /// that turn on an alternate world-line.
    pub async fn run_turn_stream(
        self: &Arc<Self>,
        convo_id: Snowflake,
        user_id: Snowflake,
        target: Option<Snowflake>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<RealmStreamEvent, RealmError>> + Send>>, RealmError>
    {
        let (convo, cast) = self.realm_conversation(convo_id, user_id).await?;

        let prep = match self.prepare_turn(&convo, cast, target).await? {
            TurnPrep::Player { agent_id } => {
                return Ok(Box::pin(futures_util::stream::once(async move {
                    Ok(RealmStreamEvent::AwaitingPlayer { agent_id })
                })));
            }
            TurnPrep::Llm(prep) => prep,
        };

        let mut open = self
            .chat
            .open_agent_turn(&prep.spec, &prep.turns, &prep.baseline, &prep.policy)
            .await?;

        let (cancel, guard) = self.chat.begin_cancel(convo_id);

        let service = Arc::clone(self);
        let stream = async_stream::try_stream! {
            let _guard = guard; // the stream owns the cancellation slot
            let prep = *prep;
            yield RealmStreamEvent::TurnStarted {
                agent_id: prep.agent.id,
                name: prep.agent.name.clone(),
            };

            let mut acc = TurnAccumulator::default();
            loop {
                // `?` can't live inside select! arms under try_stream!, so
                // resolve the branch first and handle the result out here.
                let next = tokio::select! {
                    biased;
                    _ = cancel.cancelled() => None,
                    next = open.stream.next() => next,
                };
                let Some(event) = next else { break };
                match event? {
                    InferEvent::Text(d) => {
                        acc.text(&d);
                        yield RealmStreamEvent::Delta { content: d };
                    }
                    InferEvent::Reasoning(d) => {
                        acc.reasoning(&d);
                        yield RealmStreamEvent::Reasoning { content: d };
                    }
                    InferEvent::Widget(w) => {
                        yield RealmStreamEvent::Widget { widget: w.clone() };
                        let marker = acc.widget(w);
                        yield RealmStreamEvent::Delta { content: marker };
                    }
                    InferEvent::ToolCall(r) => {
                        yield RealmStreamEvent::Action {
                            name: r.name.clone(),
                            is_error: r.is_error,
                        };
                        acc.tool_call(r);
                    }
                }
            }

            let delta = open.delta(&service.state);
            let activation = std::mem::take(&mut open.activation);
            let TurnResult { message, events } = service
                .finalize_llm_turn(prep, acc.finish(), delta, activation, open.had_assembly)
                .await?;
            yield RealmStreamEvent::Committed { message, events };
        };

        Ok(Box::pin(stream))
    }

    /// A player's turn: a user message (+ optional structured action)
    /// through the same grant-checked weaver pipeline as any LLM tool call,
    /// committed as one message with its events.
    pub async fn submit_player_turn(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        req: PlayerTurnRequest,
    ) -> Result<TurnResult, RealmError> {
        let (convo, cast) = self.realm_conversation(convo_id, user_id).await?;

        let path = self.active_path(&convo).await?;
        let agent = self
            .policy
            .next(path.len(), &cast.agents)
            .ok_or(RealmError::NoAgents(cast.realm_id))?
            .clone();

        if !matches!(agent.kind, AgentKind::Player) || agent.user_id != Some(user_id) {
            return Err(RealmError::NotYourTurn(agent.id));
        }

        let content = req.content.unwrap_or_default();
        if content.trim().is_empty() && req.action.is_none() {
            return Err(RealmError::InvalidTurn(
                "a player turn needs text, an action, or both".into(),
            ));
        }

        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let reconstructed = self.state.reconstruct(&ids).await?;

        // A player action runs the exact same weaver pipeline
        // an LLM agent's tool call goes through
        let mut delta = StateDelta::default();
        let mut actor_notes = Vec::new();
        let mut action_meta = json!(null);
        let mut acted: Option<(UserTool, AgentToolBinding, serde_json::Value)> = None;
        if let Some(action) = req.action {
            let binding = agent
                .tools
                .iter()
                .find(|b| b.tool_id == action.tool_id)
                .ok_or(RealmError::ToolNotBound(action.tool_id))?
                .clone();
            let tool = self
                .storage
                .get_tool(binding.tool_id)
                .await?
                .ok_or(RealmError::NotFound(TargetType::UserTool, binding.tool_id))?;
            let (output, d) = self
                .execute_action(
                    &tool,
                    &agent.grants,
                    action.args.clone(),
                    &reconstructed.persistent,
                )
                .await?;

            // TODO: remove hard-coded formatting
            actor_notes.push(format!("[{} result: {}]", tool.name, output));
            action_meta = json!({ "tool": tool.name, "input": action.args, "output": output });
            delta = d;
            acted = Some((tool, binding, action.args));
        }

        let meta = json!({
            "agent_id": agent.id,
            "agent_name": agent.name,
            "action": action_meta,
        });
        let message = self
            .conversations
            .append_message(convo.id, ChatRole::User, content.clone(), meta)
            .await?;

        if !delta.writes.is_empty() || !delta.deletes.is_empty() {
            // Carry the reconstructed activation forward so a player-turn
            // state row never resets lorebook activation for later turns.
            self.state
                .persist(
                    message.id,
                    CapturedState {
                        activation: reconstructed.activation.clone(),
                        persistent: delta.clone(),
                    },
                )
                .await?;
        }

        let mut events = Vec::new();
        if !content.trim().is_empty() {
            events.push(self.event(
                &convo,
                message.id,
                agent.id,
                events.len(),
                MAIN_SCOPE.to_string(),
                EventKind::Spoke,
                content,
                json!({}),
            ));
        }
        if let Some((tool, binding, args)) = acted {
            events.push(self.event(
                &convo,
                message.id,
                agent.id,
                events.len(),
                binding.event_scope.unwrap_or_default(),
                EventKind::Acted,
                // TODO: remove hard-coded formatting
                format!("[{} used {}]", agent.name, tool.name),
                json!({ "tool": tool.name, "input": args }),
            ));
        }
        self.push_state_events(&convo, message.id, agent.id, &delta, &mut events);

        let mut observations = self.fan_out(&cast.agents, &events, &agent.name);
        observations.extend(self.actor_notes(&convo, message.id, agent.id, actor_notes));

        self.storage
            .commit_turn(TurnCommit {
                events: events.clone(),
                observations,
            })
            .await?;

        Ok(TurnResult { message, events })
    }

    pub async fn cast(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmCast, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        Ok(cast)
    }

    pub async fn list_events(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        before: Option<Snowflake>,
    ) -> Result<Vec<RealmEvent>, RealmError> {
        self.realm_conversation(convo_id, user_id).await?;
        let events = self
            .storage
            .list_realm_events(convo_id, before, EVENT_WINDOW)
            .await?;

        Ok(events)
    }
    
    pub async fn list_observations(
        &self,
        convo_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        before: Option<Snowflake>,
    ) -> Result<Vec<AgentObservation>, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        self.cast_agent(&cast, agent_id)?;
        let observations = self
            .storage
            .list_observations(convo_id, agent_id, before, OBSERVATION_WINDOW)
            .await?;

        Ok(observations)
    }

    pub async fn agent_state_slice(
        &self,
        convo_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<StatePool, RealmError> {
        let (convo, cast) = self.realm_conversation(convo_id, user_id).await?;
        let agent = self.cast_agent(&cast, agent_id)?.clone();

        let path = self.active_path(&convo).await?;
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&ids).await?.persistent;

        let read_scopes: Vec<ScopePattern> = agent
            .grants
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();

        Ok(world
            .into_iter()
            .filter(|(ns, _)| read_scopes.iter().any(|g| g.matches(ns)))
            .collect())
    }

    pub async fn conversation_actions(
        &self,
        convo_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        let agent = self.cast_agent(&cast, agent_id)?;

        self.action_specs(&agent.tools).await
    }
}

impl RealmService {
    async fn prepare_turn(
        &self,
        convo: &Conversation,
        cast: RealmCast,
        target: Option<Snowflake>,
    ) -> Result<TurnPrep, RealmError> {
        let (path, persist) = match target {
            None => {
                let path = self.active_path(convo).await?;
                (path, PersistPlan::Append(convo.active_leaf_id))
            }
            Some(t) => {
                let msg = self
                    .storage
                    .get_message(t)
                    .await?
                    .filter(|m| m.conversation_id == convo.id)
                    .ok_or(RealmError::NotFound(TargetType::Message, t))?;
                let path = match msg.parent {
                    Some(p) => self.storage.get_path_to(convo.id, p).await?,
                    None => Vec::new(),
                };
                (path, PersistPlan::Alternate(t))
            }
        };

        let agent = self
            .policy
            .next(path.len(), &cast.agents)
            .ok_or(RealmError::NoAgents(cast.realm_id))?
            .clone();

        if matches!(agent.kind, AgentKind::Player) {
            return Ok(TurnPrep::Player { agent_id: agent.id });
        }

        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let reconstructed = self.state.reconstruct(&ids).await?;

        let read_scopes: Vec<ScopePattern> = agent
            .grants
            .iter()
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let write_scopes: Vec<ScopePattern> = agent
            .grants
            .iter()
            .filter(|g| matches!(g.access, AgentAccess::ReadWrite))
            .map(|g| ScopePattern::parse(&g.scope))
            .collect();
        let slice: StatePool = reconstructed
            .persistent
            .iter()
            .filter(|(ns, _)| read_scopes.iter().any(|g| g.matches(ns)))
            .map(|(ns, vars)| (ns.clone(), vars.clone()))
            .collect();

        let mut observations = self
            .storage
            .list_observations_for_messages(agent.id, &ids)
            .await?;

        if observations.len() > OBSERVATION_WINDOW as usize {
            observations.drain(..observations.len() - OBSERVATION_WINDOW as usize);
        }
        let turns: Vec<PromptTurn> = observations
            .iter()
            .map(PromptTurn::from_observation)
            .collect();

        let spec = self.spec_for(convo.user_id, cast.tokenizer_id, &agent)?;
        let baseline = ReconstructedState {
            persistent: slice,
            activation: reconstructed.activation,
        };
        let policy = ToolPolicy {
            write_scopes: Some(Arc::new(write_scopes)),
            tool_ids: Some(agent.tools.iter().map(|b| b.tool_id).collect()),
        };

        Ok(TurnPrep::Llm(Box::new(PreparedTurn {
            convo: convo.clone(),
            cast,
            agent,
            spec,
            baseline,
            policy,
            persist,
            turns,
        })))
    }

    async fn finalize_llm_turn(
        &self,
        prep: PreparedTurn,
        outcome: AgentOutcome,
        delta: StateDelta,
        activation: HashMap<Snowflake, ActivationState>,
        had_assembly: bool,
    ) -> Result<TurnResult, RealmError> {
        let PreparedTurn {
            convo,
            cast,
            agent,
            persist,
            ..
        } = prep;

        let meta = json!({
            "reasoning": outcome.reasoning,
            "tool_calls": outcome.tool_calls,
            "widgets": outcome.widgets,
            "agent_id": agent.id,
            "agent_name": agent.name,
        });

        let message = match persist {
            PersistPlan::Append(Some(parent)) => {
                self.conversations
                    .branch_from(
                        convo.id,
                        parent,
                        ChatRole::Assistant,
                        outcome.spoken.clone(),
                        meta,
                    )
                    .await?
            }
            PersistPlan::Append(None) => {
                self.conversations
                    .append_message(convo.id, ChatRole::Assistant, outcome.spoken.clone(), meta)
                    .await?
            }
            PersistPlan::Alternate(sibling) => {
                self.conversations
                    .create_alternate(
                        convo.id,
                        sibling,
                        ChatRole::Assistant,
                        outcome.spoken.clone(),
                        meta,
                    )
                    .await?
            }
        };

        if had_assembly || !delta.writes.is_empty() || !delta.deletes.is_empty() {
            self.state
                .persist(
                    message.id,
                    CapturedState {
                        activation,
                        persistent: delta.clone(),
                    },
                )
                .await?;
        }

        let action_scopes = self.action_scopes(&agent.tools).await?;

        let mut events = Vec::new();
        if !outcome.spoken.trim().is_empty() {
            events.push(self.event(
                &convo,
                message.id,
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
                &convo,
                message.id,
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

        self.push_state_events(&convo, message.id, agent.id, &delta, &mut events);

        let mut observations = self.fan_out(&cast.agents, &events, &agent.name);
        observations.extend(self.actor_notes(&convo, message.id, agent.id, actor_notes));

        self.storage
            .commit_turn(TurnCommit {
                events: events.clone(),
                observations,
            })
            .await?;

        Ok(TurnResult { message, events })
    }

    async fn realm_conversation(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(Conversation, RealmCast), RealmError> {
        let convo = self
            .storage
            .get_conversation(convo_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::Conversation, convo_id))?;

        if convo.realm_id.is_none() {
            return Err(RealmError::NotRealmConversation(convo_id));
        }
        let cast = self
            .storage
            .get_cast(convo_id)
            .await?
            .ok_or(RealmError::NotRealmConversation(convo_id))?;

        let participant =
            convo.user_id == user_id || cast.agents.iter().any(|a| a.user_id == Some(user_id));
        if !participant {
            return Err(RealmError::NotOwned(TargetType::Conversation, convo_id));
        }

        Ok((convo, cast))
    }

    fn cast_agent<'a>(
        &self,
        cast: &'a RealmCast,
        agent_id: Snowflake,
    ) -> Result<&'a CastAgent, RealmError> {
        cast.agents
            .iter()
            .find(|a| a.id == agent_id)
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))
    }

    async fn active_path(
        &self,
        convo: &Conversation,
    ) -> Result<Vec<lorealm_storage::models::ConversationMessage>, RealmError> {
        Ok(match convo.active_leaf_id {
            Some(leaf) => self.storage.get_path_to(convo.id, leaf).await?,
            None => Vec::new(),
        })
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

    async fn validate_agent(
        &self,
        user_id: Snowflake,
        req: &CreateAgentRequest,
    ) -> Result<AgentParts, RealmError> {
        let parts = match req.kind {
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
                AgentParts {
                    character_id: None,
                    char_version_id: None,
                    template_id: None,
                    profile_id: None,
                    user_id: Some(user_id),
                }
            }
            AgentKind::Llm => {
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

                let template_id = match req.template_id {
                    Some(id) => self.templates.get_template(id, user_id).await?.id,
                    None => self
                        .state
                        .get_active_template_id(user_id)
                        .await?
                        .ok_or(RealmError::NotFound(TargetType::ContextTemplate, user_id))?,
                };
                let profile_id = match req.profile_id {
                    Some(id) => self.connection.get_profile(user_id, id).await?.id,
                    None => self.state.get_active_profile(user_id).await?.id,
                };

                AgentParts {
                    character_id: character.0,
                    char_version_id: character.1,
                    template_id: Some(template_id),
                    profile_id: Some(profile_id),
                    user_id: None,
                }
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

        Ok(parts)
    }

    async fn apply_agent_bindings(
        &self,
        agent_id: Snowflake,
        grants: Vec<GrantDto>,
        tools: Vec<ToolBindingDto>,
    ) -> Result<(), RealmError> {
        let grants: Vec<AgentGrant> = grants
            .into_iter()
            .map(|g| AgentGrant {
                agent_id,
                scope: g.scope,
                access: g.access,
            })
            .collect();
        self.storage.set_agent_grants(agent_id, &grants).await?;

        let bindings: Vec<AgentToolBinding> = tools
            .iter()
            .map(|t| AgentToolBinding {
                tool_id: t.tool_id,
                event_scope: t.event_scope.clone(),
            })
            .collect();
        self.storage.set_agent_tools(agent_id, &bindings).await?;

        Ok(())
    }

    fn fan_out(
        &self,
        agents: &[CastAgent],
        events: &[RealmEvent],
        source_name: &str,
    ) -> Vec<AgentObservation> {
        let mut out = Vec::new();
        for observer in agents {
            let grants: Vec<ScopePattern> = observer
                .grants
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
                    conversation_id: event.conversation_id,
                    message_id: event.message_id,
                    agent_id: observer.id,
                    event_id: Some(event.id),
                    role,
                    content,
                });
            }
        }
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn event(
        &self,
        convo: &Conversation,
        message_id: Snowflake,
        source: Snowflake,
        seq: usize,
        scope: String,
        kind: EventKind,
        content: String,
        payload: serde_json::Value,
    ) -> RealmEvent {
        RealmEvent {
            id: self.snowflake.generate(),
            conversation_id: convo.id,
            message_id,
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
        convo: &Conversation,
        message_id: Snowflake,
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
                convo,
                message_id,
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

    async fn action_specs(
        &self,
        bindings: &[AgentToolBinding],
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let mut out = Vec::new();
        for binding in bindings {
            let Some(tool) = self.storage.get_tool(binding.tool_id).await? else {
                continue;
            };
            out.push(ActionSpec {
                tool_id: tool.id,
                name: tool.name,
                display_name: tool.display_name,
                description: tool.description,
                params: tool.params,
                event_scope: binding.event_scope.clone(),
            });
        }
        Ok(out)
    }

    fn actor_notes(
        &self,
        convo: &Conversation,
        message_id: Snowflake,
        agent_id: Snowflake,
        notes: Vec<String>,
    ) -> Vec<AgentObservation> {
        notes
            .into_iter()
            .map(|content| AgentObservation {
                id: self.snowflake.generate(),
                conversation_id: convo.id,
                message_id,
                agent_id,
                event_id: None,
                role: ChatRole::User,
                content,
            })
            .collect()
    }

    async fn execute_action(
        &self,
        tool: &UserTool,
        grants: &[CastGrant],
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

    fn spec_for(
        &self,
        owner: Snowflake,
        tokenizer_id: Snowflake,
        agent: &CastAgent,
    ) -> Result<AgentSpec, RealmError> {
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
            owner,
            character,
            template_id,
            persona_id: agent.persona_id,
            tokenizer_id,
            profile_id,
        })
    }
}

/// The identity bundle a validated create/update agent request resolves to.
struct AgentParts {
    character_id: Option<Snowflake>,
    char_version_id: Option<Snowflake>,
    template_id: Option<Snowflake>,
    profile_id: Option<Snowflake>,
    user_id: Option<Snowflake>,
}

/// The resolved turn before inference runs.
enum TurnPrep {
    Player {
        agent_id: Snowflake,
    },
    Llm(Box<PreparedTurn>),
}

enum PersistPlan {
    /// Continue the active path (None = the playthrough's first message).
    Append(Option<Snowflake>),
    /// Regenerate: a new sibling of this message.
    Alternate(Snowflake),
}

struct PreparedTurn {
    convo: Conversation,
    cast: RealmCast,
    agent: CastAgent,
    spec: AgentSpec,
    baseline: ReconstructedState,
    policy: ToolPolicy,
    persist: PersistPlan,
    turns: Vec<PromptTurn>,
}
