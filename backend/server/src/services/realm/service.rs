use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use lorealm_providers::Tool;
use lorealm_storage::{
    Storage,
    models::{
        ActivationState, AgentAccess, AgentGrant, AgentKind, AgentObservation, AgentSpec,
        AgentToolBinding, ChatRole, Conversation, EventKind, Realm, RealmAgent, RealmCast,
        RealmDefinition, RealmEvent, RealmSummary, RealmVersionInfo, ScopePattern, Snowflake,
        StateDelta, TurnCommit, UserTool, VersionedSaveMode,
    },
};
use serde_json::json;
use tokio::{sync::RwLock, time::Instant};

use crate::{
    core::CleanableService,
    errors::{RealmError, TargetType},
    routes::{CreateAgentRequest, PlayerTurnRequest, UpdateRealmRequest},
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

/// Edits within this window collapse into the same version (mirrors the
/// character editing sessions and their default TTL).
const EDIT_SESSION_TTL: Duration = Duration::from_secs(600);

struct EditingSession {
    version_id: Snowflake,
    last_touched: Instant,
}

/// A realm loaded for mutation plus how to persist it: within an editing
/// session the current version is updated in place, otherwise a new
/// version row is appended.
struct EditableRealm {
    realm: Realm,
    save_mode: VersionedSaveMode,
}

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
    editing_sessions: RwLock<HashMap<Snowflake, EditingSession>>,
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
            editing_sessions: RwLock::new(HashMap::new()),
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
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        let realm = Realm {
            id: self.snowflake.generate(),
            owner_id: user_id,
            current_version_id: self.snowflake.generate(),
            definition: RealmDefinition {
                name: name.into(),
                ..Default::default()
            },
            tags: Vec::new(),
        };

        self.storage
            .save_realm(&realm, VersionedSaveMode::NewVersion)
            .await?;

        Ok(realm)
    }

    pub async fn update_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: UpdateRealmRequest,
    ) -> Result<Realm, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        if let Some(name) = req.name {
            edit.realm.definition.name = name;
        }
        if let Some(description) = req.description {
            edit.realm.definition.description = description;
        }
        if let Some(tags) = req.tags {
            edit.realm.tags = tags;
        }

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(edit.realm)
    }

    pub async fn delete_realm(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        self.storage.delete_realm(realm_id).await?;
        self.editing_sessions.write().await.remove(&realm_id);

        Ok(())
    }

    pub async fn list_versions(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmVersionInfo>, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        Ok(self.storage.list_realm_versions(realm_id).await?)
    }

    pub async fn get_version(
        &self,
        realm_id: Snowflake,
        version_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        self.storage
            .get_realm_at_version(realm_id, version_id)
            .await?
            .ok_or(RealmError::NotFound(TargetType::RealmVersion, version_id))
    }

    /// Roll the head back to a prior version (as a fresh version row, so
    /// history stays linear and nothing is lost).
    pub async fn restore_version(
        &self,
        realm_id: Snowflake,
        version_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Realm, RealmError> {
        self.owned_realm(realm_id, user_id).await?;

        self.storage
            .restore_realm_version(realm_id, version_id, self.snowflake.generate())
            .await?;
        // The next edit must not collapse into a pre-restore session.
        self.editing_sessions.write().await.remove(&realm_id);

        self.owned_realm(realm_id, user_id).await
    }

    pub async fn list_agents(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmAgent>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;

        Ok(realm.definition.agents)
    }

    pub async fn list_agent_grants(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        let agent = Self::definition_agent(&realm.definition, agent_id)?;

        Ok(agent.grants.clone())
    }

    /// The agent's action space as currently authored in the preset.
    pub async fn list_actions(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        let agent = Self::definition_agent(&realm.definition, agent_id)?;

        self.action_specs(&agent.tools).await
    }

    pub async fn create_agent(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let parts = self.validate_agent(user_id, &req).await?;

        let agent = RealmAgent {
            id: self.snowflake.generate(),
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            turn_order: edit.realm.definition.agents.len() as i32,
            grants: req.grants,
            tools: req.tools,
        };

        edit.realm.definition.agents.push(agent.clone());
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(agent)
    }

    /// Full-replace edit of a preset agent (grants and tools included).
    /// In-flight playthroughs keep their cast snapshot; only new ones see
    /// this.
    pub async fn update_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let parts = self.validate_agent(user_id, &req).await?;

        let slot = edit
            .realm
            .definition
            .agents
            .iter_mut()
            .find(|a| a.id == agent_id)
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))?;

        let agent = RealmAgent {
            id: slot.id,
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            turn_order: slot.turn_order,
            grants: req.grants,
            tools: req.tools,
        };
        *slot = agent.clone();

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(agent)
    }

    pub async fn delete_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agents = &mut edit.realm.definition.agents;
        let before = agents.len();
        agents.retain(|a| a.id != agent_id);
        if agents.len() == before {
            return Err(RealmError::NotFound(TargetType::RealmAgent, agent_id));
        }
        for (index, agent) in agents.iter_mut().enumerate() {
            agent.turn_order = index as i32;
        }

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(())
    }

    pub async fn set_grants(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        grants: Vec<AgentGrant>,
    ) -> Result<(), RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agent = edit
            .realm
            .definition
            .agents
            .iter_mut()
            .find(|a| a.id == agent_id)
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))?;
        agent.grants = grants;

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(())
    }

    // -------------------------------------------------------- playthroughs

    /// Instantiate a realm: create a conversation bound to it and pin the
    /// cast snapshot from the current version. No seed state is copied —
    /// there is none; the opening pool is emergent, written by the weaver
    /// expressions the realm's lorebooks/tools run over the first turns.
    pub async fn start_conversation(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<FullConversation, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;

        if realm.definition.agents.is_empty() {
            return Err(RealmError::NoAgents(realm_id));
        }
        let mut agents = realm.definition.agents;
        agents.sort_by_key(|a| (a.turn_order, a.id));

        let convo = Conversation {
            id: self.snowflake.generate(),
            user_id,
            character_id: None,
            char_version_id: None,
            realm_id: Some(realm_id),
            active_leaf_id: None,
            title: Some(realm.definition.name.clone()),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        self.storage.create_conversation(&convo).await?;
        self.storage
            .put_cast(
                convo.id,
                &RealmCast {
                    realm_id,
                    version_id: Some(realm.current_version_id),
                    agents,
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

    // ------------------------------------------------- playthrough views

    /// The cast snapshot of a playthrough (roster, turn order, bindings).
    pub async fn cast(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmCast, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        Ok(cast)
    }

    /// The playthrough's objective timeline (owner view).
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

    /// An agent's private feed within a playthrough.
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

    /// The agent's grant-filtered view of the pool at the active leaf.
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

    /// The agent's action space as pinned in the playthrough's snapshot.
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

// ---------------------------------------------------------------- internals

impl RealmService {
    /// Load a realm for mutation, resolving the editing session: edits
    /// within the TTL window collapse into the same version, otherwise a
    /// fresh version id is allocated (mirrors character editing).
    async fn get_realm_for_edit(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<EditableRealm, RealmError> {
        let mut realm = self.owned_realm(realm_id, user_id).await?;

        let (mode, target) = {
            let mut sessions = self.editing_sessions.write().await;
            match sessions.get_mut(&realm.id) {
                Some(s) if s.last_touched.elapsed() < EDIT_SESSION_TTL => {
                    s.last_touched = Instant::now();
                    (VersionedSaveMode::UpdateCurrent, s.version_id)
                }
                _ => {
                    let v = self.snowflake.generate();
                    sessions.insert(
                        realm.id,
                        EditingSession {
                            version_id: v,
                            last_touched: Instant::now(),
                        },
                    );
                    (VersionedSaveMode::NewVersion, v)
                }
            }
        };

        realm.current_version_id = target;
        Ok(EditableRealm {
            realm,
            save_mode: mode,
        })
    }

    /// Everything up to (but excluding) inference: resolve whose turn it is
    /// along the path and project that agent's view of the world.
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

        // Turn ≈ message: the turn index is the position along the path, so
        // a regenerated sibling re-runs the same agent's turn.
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
        // Path order == id order: along one path every message commits after
        // its ancestors, and observations are created with their message.
        if observations.len() > OBSERVATION_WINDOW as usize {
            observations.drain(..observations.len() - OBSERVATION_WINDOW as usize);
        }
        let turns: Vec<PromptTurn> = observations
            .iter()
            .map(PromptTurn::from_observation)
            .collect();

        // Realms don't own a tokenizer; turns use the conversation owner's
        // active one, exactly like plain chat.
        let tokenizer_id = self.state.get_active_tokenizer(convo.user_id).await?.id;
        let spec = self.spec_for(convo.user_id, tokenizer_id, &agent)?;
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

    /// Turn an LLM outcome into a committed message + state delta + events.
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
            // First turn of the playthrough: the message becomes the root.
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

    /// Load + authorize a realm playthrough: the caller must own the
    /// conversation or drive one of its player agents.
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
    ) -> Result<&'a RealmAgent, RealmError> {
        cast.agents
            .iter()
            .find(|a| a.id == agent_id)
            .ok_or(RealmError::NotFound(TargetType::RealmAgent, agent_id))
    }

    fn definition_agent(
        definition: &RealmDefinition,
        agent_id: Snowflake,
    ) -> Result<&RealmAgent, RealmError> {
        definition
            .agents
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

    /// Validate the identity bundle of a create/update agent request.
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

    /// Fan events out into per-agent observations, strictly through the
    /// snapshot grants. This is the enforcement point for knowledge
    /// asymmetry: no grant on the scope, no memory of the event.
    fn fan_out(
        &self,
        agents: &[RealmAgent],
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

    fn spec_for(
        &self,
        owner: Snowflake,
        tokenizer_id: Snowflake,
        agent: &RealmAgent,
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

#[async_trait]
impl CleanableService for RealmService {
    fn name(&self) -> &'static str {
        "RealmService"
    }
    async fn clean(&self) -> usize {
        let mut sessions = self.editing_sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| s.last_touched.elapsed() < EDIT_SESSION_TTL);
        before - sessions.len()
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
    /// A player is up; nothing to run server-side.
    Player {
        agent_id: Snowflake,
    },
    Llm(Box<PreparedTurn>),
}

/// Where the committed assistant message lands in the tree.
enum PersistPlan {
    /// Continue the active path (None = the playthrough's first message).
    Append(Option<Snowflake>),
    /// Regenerate: a new sibling of this message.
    Alternate(Snowflake),
}

/// An LLM agent's turn, ready to run: its identity bundle, its projected
/// slice of the world, and the policy constraining its tools.
struct PreparedTurn {
    convo: Conversation,
    cast: RealmCast,
    agent: RealmAgent,
    spec: AgentSpec,
    baseline: ReconstructedState,
    policy: ToolPolicy,
    persist: PersistPlan,
    turns: Vec<PromptTurn>,
}
