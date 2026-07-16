use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use lorealm_storage::{
    Storage,
    models::{
        ActivationState, AgentGrant, AgentKind, AgentObservation, AgentSpec, AgentToolBinding,
        ChatRole, Conversation, ConversationMessage, EventKind, ImportRule, Realm, RealmAgent,
        RealmCast, RealmConfig, RealmDefinition, RealmEvent, RealmSummary, RealmVersionInfo,
        ResolvedRealmConfig, Snowflake, StateDelta, TurnCommit, UserTool, VersionedSaveMode,
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
        context::{ContextService, TracedBlock},
        conversation::ConversationService,
        persona::PersonaService,
        realm::{ActionSpec, RealmStreamEvent, RoundRobin, TurnPolicy, TurnResult},
        snowflake::SnowflakeService,
        state::{CapturedState, StatePool, StateService},
    },
};

use super::{
    agent::{
        AgentRegistry, EventDraft, PersistPlan, PlanCx, Production, ReadyTurn, TurnPlan, capture,
        push_state_events, speech_scope,
    },
    validate::{agent_required_imports, apply_turn_order},
    weaver,
};

const EVENT_WINDOW: i64 = 500;

/// Edits within this window collapse into the same version.
const EDIT_SESSION_TTL: Duration = Duration::from_secs(600);

struct EditingSession {
    version_id: Snowflake,
    last_touched: Instant,
}

/// A realm loaded for mutation plus how to persist it: within an editing
/// session the current version is updated in place, otherwise a new version
/// row is appended.
struct EditableRealm {
    realm: Realm,
    save_mode: VersionedSaveMode,
}

pub struct RealmService {
    pub(super) storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    chat: Arc<ChatService>,
    pub(super) characters: Arc<CharacterService>,
    pub(super) templates: Arc<ContextService>,
    pub(super) connection: Arc<ConnectionService>,
    pub(super) personas: Arc<PersonaService>,
    pub(super) state: Arc<StateService>,
    conversations: Arc<ConversationService>,
    policy: Arc<dyn TurnPolicy>,
    agents: AgentRegistry,
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
            agents: AgentRegistry::with_defaults(),
            editing_sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn list_realms(&self, user_id: Snowflake) -> Result<Vec<RealmSummary>, RealmError> {
        let realms = self.storage.list_realms(user_id).await?;

        Ok(realms)
    }

    pub async fn get_config(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmConfig, RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        Ok(self.storage.get_realm_config(realm_id).await?)
    }

    pub async fn set_config(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        config: RealmConfig,
    ) -> Result<RealmConfig, RealmError> {
        self.owned_realm(realm_id, user_id).await?;
        self.storage
            .put_realm_config(realm_id, user_id, &config)
            .await?;
        Ok(config)
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

        let parts = self
            .validate_agent(user_id, &req, &edit.realm.definition.imports)
            .await?;

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
            speech_scope: req.speech_scope,
            lorebooks: req.lorebooks,
            script: req.script,
            run_condition: req.run_condition,
        };

        edit.realm.definition.agents.push(agent.clone());
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(agent)
    }

    /// Full-replace edit of a preset agent (grants and tools included).
    pub async fn update_agent(
        &self,
        realm_id: Snowflake,
        agent_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let parts = self
            .validate_agent(user_id, &req, &edit.realm.definition.imports)
            .await?;

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
            speech_scope: req.speech_scope,
            lorebooks: req.lorebooks,
            script: req.script,
            run_condition: req.run_condition,
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

    /// Rewrite the cast's turn order. `order` must be a permutation of the
    /// current agent ids; round-robin then runs in exactly this sequence.
    pub async fn reorder_agents(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        order: Vec<Snowflake>,
    ) -> Result<Vec<RealmAgent>, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agents = &mut edit.realm.definition.agents;
        apply_turn_order(agents, &order)?;

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(edit.realm.definition.agents)
    }

    /// Full-replace edit of the realm's import manifest. Every rule must
    /// point at an existing resource the caller owns, and no agent may be
    /// left referencing something the new list no longer covers.
    pub async fn set_imports(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        imports: Vec<ImportRule>,
    ) -> Result<Vec<ImportRule>, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let mut seen = HashSet::new();
        let imports: Vec<ImportRule> = imports
            .into_iter()
            .filter(|r| seen.insert((r.kind, r.resource_id)))
            .collect();

        for rule in &imports {
            self.validate_import(user_id, rule).await?;
        }

        for agent in &edit.realm.definition.agents {
            for (kind, id) in agent_required_imports(agent) {
                if !seen.contains(&(kind, id)) {
                    return Err(RealmError::ImportInUse(kind, id, agent.name.clone()));
                }
            }
        }

        edit.realm.definition.imports = imports.clone();
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(imports)
    }

    // -------------------------------------------------------- playthroughs

    /// Instantiate a realm: create a conversation bound to it and pin the
    /// cast snapshot from the current version. No seed state is stored; the
    /// opening pool is written by the weaver expressions its lorebooks/tools
    /// run over the first turns.
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
        let config = self.resolve_config(&convo).await?;

        let llm = match self.plan_turn(&convo, cast, target).await? {
            TurnPlan::AwaitPlayer { agent_id } => {
                return Ok(Box::pin(futures_util::stream::once(async move {
                    Ok(RealmStreamEvent::AwaitingPlayer { agent_id })
                })));
            }
            TurnPlan::Ready(ready) => {
                let service = Arc::clone(self);
                let config = config.clone();
                let stream = async_stream::try_stream! {
                    let ReadyTurn { convo, cast, production } = *ready;
                    yield RealmStreamEvent::TurnStarted {
                        agent_id: production.agent.id,
                        name: production.agent.name.clone(),
                    };
                    let TurnResult { message, events } =
                        service.commit(&convo, &cast, &config, production).await?;
                    yield RealmStreamEvent::Committed { message, events };
                };
                return Ok(Box::pin(stream));
            }
            TurnPlan::Stream(llm) => llm,
        };

        let mut open = self
            .chat
            .open_agent_turn(&llm.spec, &llm.turns, &llm.baseline, &llm.policy)
            .await?;

        let (cancel, guard) = self.chat.begin_cancel(convo_id);

        let service = Arc::clone(self);
        let stream = async_stream::try_stream! {
            let _guard = guard; // the stream owns the cancellation slot
            let llm = *llm;
            yield RealmStreamEvent::TurnStarted {
                agent_id: llm.agent.id,
                name: llm.agent.name.clone(),
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
            let trace = std::mem::take(&mut open.trace);
            let production = service
                .llm_production(&llm.agent, &llm.spec, trace, llm.persist, acc.finish(), delta, activation, open.had_assembly, &config)
                .await?;
            let TurnResult { message, events } =
                service.commit(&llm.convo, &llm.cast, &config, production).await?;
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
        let config = self.resolve_config(&convo).await?;

        let path = self.active_path(&convo).await?;
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let reconstructed = self.state.reconstruct(&ids).await?;

        // The player is only up if the run_condition skip-forward scan lands
        // on them against this world.
        let agent = self
            .runnable_agent(path.len(), &cast.agents, &reconstructed.persistent)
            .ok_or(RealmError::NoRunnableAgent)?
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

        // A player action runs the same weaver pipeline as an LLM tool call.
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
            let (output, d) = weaver::run_scoped(
                &self.state,
                &tool.body,
                &tool.params,
                action.args.clone(),
                &agent.grants,
                &reconstructed.persistent,
            )
            .await?;

            actor_notes.push(config.tool_result(&tool.name, &output));
            action_meta = json!({ "tool": tool.name, "input": action.args, "output": output });
            delta = d;
            acted = Some((tool, binding, action.args));
        }

        let meta = json!({
            "agent_id": agent.id,
            "agent_name": agent.name,
            "action": action_meta,
        });

        let mut events = Vec::new();
        if !content.trim().is_empty() {
            events.push(EventDraft {
                source: agent.id,
                scope: speech_scope(&agent),
                kind: EventKind::Spoke,
                content: content.clone(),
                payload: json!({}),
            });
        }
        if let Some((tool, binding, args)) = acted {
            events.push(EventDraft {
                source: agent.id,
                scope: binding.event_scope.unwrap_or_default(),
                kind: EventKind::Acted,
                content: config.action(&agent.name, &tool.name),
                payload: json!({ "tool": tool.name, "input": args }),
            });
        }
        push_state_events(agent.id, &delta, &config, &mut events);

        // Carry the reconstructed activation forward so a player-turn state
        // row never resets lorebook activation for later turns.
        let production = Production {
            agent,
            persist: PersistPlan::Append(convo.active_leaf_id),
            role: ChatRole::User,
            content,
            meta,
            state: capture(reconstructed.activation, delta),
            events,
            actor_notes,
        };

        self.commit(&convo, &cast, &config, production).await
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
            .list_observations(convo_id, agent_id, before, super::OBSERVATION_WINDOW)
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

        Ok(weaver::slice(&world, &weaver::read_scopes(&agent.grants)))
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

    /// Resolve whose turn it is along the path and project that agent's view
    /// of the world into an executable plan (dispatched by kind).
    async fn plan_turn(
        &self,
        convo: &Conversation,
        cast: RealmCast,
        target: Option<Snowflake>,
    ) -> Result<TurnPlan, RealmError> {
        let (path, persist) = self.resolve_path(convo, target).await?;

        let path_ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&path_ids).await?;

        // The turn index is the position along the path, so a regenerated
        // sibling re-runs the same agent's turn.
        let agent = self
            .runnable_agent(path.len(), &cast.agents, &world.persistent)
            .ok_or(RealmError::NoRunnableAgent)?
            .clone();

        self.agents
            .get(&agent.kind)
            .plan(PlanCx {
                service: self,
                convo,
                cast,
                agent,
                path_ids,
                world,
                persist,
            })
            .await
    }

    /// Whose turn it is once run_conditions are honored: the first agent the
    /// turn policy yields from `turn_index` forward whose `run_condition`
    /// passes against `world`. `None` when every agent is gated off this turn.
    fn runnable_agent<'a>(
        &self,
        turn_index: usize,
        agents: &'a [RealmAgent],
        world: &StatePool,
    ) -> Option<&'a RealmAgent> {
        (0..agents.len()).find_map(|k| {
            let a = self.policy.next(turn_index + k, agents)?;
            weaver::run_condition_passes(&a.run_condition, &a.grants, world).then_some(a)
        })
    }

    /// The message path to assemble against and where a new message lands.
    /// `target` regenerates: the message becomes a sibling of `target`, so
    /// assembly runs against `target`'s parent path.
    async fn resolve_path(
        &self,
        convo: &Conversation,
        target: Option<Snowflake>,
    ) -> Result<(Vec<ConversationMessage>, PersistPlan), RealmError> {
        Ok(match target {
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
        })
    }

    /// Build the production for a finished LLM turn: its spoken message plus
    /// the spoke/acted events and actor notes derived from the outcome.
    async fn llm_production(
        &self,
        agent: &RealmAgent,
        spec: &AgentSpec,
        trace: Vec<TracedBlock>,
        persist: PersistPlan,
        outcome: AgentOutcome,
        delta: StateDelta,
        activation: HashMap<Snowflake, ActivationState>,
        had_assembly: bool,
        config: &ResolvedRealmConfig,
    ) -> Result<Production, RealmError> {
        // Unlike plain chat, the trace keeps its content inline: realm history
        // blocks come from per-agent observations (never message refs), and
        // character parts belong to the agent's character, which the client
        // has no active character to reconstruct them from.
        let meta = json!({
            "context": trace,
            "reasoning": outcome.reasoning,
            "tool_calls": outcome.tool_calls,
            "widgets": outcome.widgets,
            "agent_id": agent.id,
            "agent_name": agent.name,
            "conn_profile": spec.profile_id,
            "tok_id": spec.tokenizer_id,
            "template_id": spec.template_id,
        });

        let action_scopes = self.action_scopes(&agent.tools).await?;

        let mut events = Vec::new();
        if !outcome.spoken.trim().is_empty() {
            events.push(EventDraft {
                source: agent.id,
                scope: speech_scope(agent),
                kind: EventKind::Spoke,
                content: outcome.spoken.clone(),
                payload: json!({}),
            });
        }

        let mut actor_notes = Vec::new();
        for call in &outcome.tool_calls {
            let Some(scope) = action_scopes.get(&call.name) else {
                continue;
            };
            if call.is_error {
                continue;
            }
            events.push(EventDraft {
                source: agent.id,
                scope: scope.clone().unwrap_or_default(),
                kind: EventKind::Acted,
                content: config.action(&agent.name, &call.name),
                payload: json!({ "tool": call.name, "input": call.input }),
            });
            actor_notes.push(config.tool_result(&call.name, &call.output.to_string()));
        }

        push_state_events(agent.id, &delta, config, &mut events);

        // Assembly writes are how a realm's opening state materializes, so
        // persist a row even on an empty delta when assembly captured state.
        let state =
            (had_assembly || !delta.writes.is_empty() || !delta.deletes.is_empty()).then(|| {
                CapturedState {
                    activation,
                    persistent: delta,
                }
            });

        Ok(Production {
            agent: agent.clone(),
            persist,
            role: ChatRole::Assistant,
            content: outcome.spoken,
            meta,
            state,
            events,
            actor_notes,
        })
    }

    /// Persist a produced turn: the message, its optional state row, and the
    /// events + observations that fan out of it. The single commit path for
    /// LLM, script, and player turns.
    async fn commit(
        &self,
        convo: &Conversation,
        cast: &RealmCast,
        config: &ResolvedRealmConfig,
        production: Production,
    ) -> Result<TurnResult, RealmError> {
        let Production {
            agent,
            persist,
            role,
            content,
            meta,
            state,
            events,
            actor_notes,
        } = production;

        let message = self
            .persist_message(convo, persist, role, content, meta)
            .await?;

        if let Some(captured) = state {
            self.state.persist(message.id, captured).await?;
        }

        // Anchor the drafts to the committed message; seq follows push order.
        let events: Vec<RealmEvent> = events
            .into_iter()
            .enumerate()
            .map(|(seq, d)| RealmEvent {
                id: self.snowflake.generate(),
                conversation_id: convo.id,
                message_id: message.id,
                seq: seq as i32,
                source: Some(d.source),
                scope: d.scope,
                kind: d.kind,
                content: d.content,
                payload: d.payload,
            })
            .collect();

        let mut observations = self.fan_out(&cast.agents, &events, &agent.name, config);
        observations.extend(self.actor_notes(convo, message.id, agent.id, actor_notes));

        self.storage
            .commit_turn(TurnCommit {
                events: events.clone(),
                observations,
            })
            .await?;

        Ok(TurnResult { message, events })
    }

    /// Write the turn's message into the tree at the planned position.
    async fn persist_message(
        &self,
        convo: &Conversation,
        persist: PersistPlan,
        role: ChatRole,
        content: String,
        meta: serde_json::Value,
    ) -> Result<ConversationMessage, RealmError> {
        Ok(match persist {
            PersistPlan::Append(Some(parent)) => {
                self.conversations
                    .branch_from(convo.id, parent, role, content, meta)
                    .await?
            }
            // First turn of the playthrough: the message becomes the root.
            PersistPlan::Append(None) => {
                self.conversations
                    .append_message(convo.id, role, content, meta)
                    .await?
            }
            PersistPlan::Alternate(sibling) => {
                self.conversations
                    .create_alternate(convo.id, sibling, role, content, meta)
                    .await?
            }
        })
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

    /// Resolve the settings in effect for a playthrough: the realm's overrides
    /// over the built-in defaults.
    pub(super) async fn resolve_config(
        &self,
        convo: &Conversation,
    ) -> Result<ResolvedRealmConfig, RealmError> {
        let config = match convo.realm_id {
            Some(id) => self.storage.get_realm_config(id).await?,
            None => RealmConfig::default(),
        };
        Ok(ResolvedRealmConfig::resolve(&config))
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
    ) -> Result<Vec<ConversationMessage>, RealmError> {
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

    /// Fan events out into per-agent observations, filtered by the snapshot
    /// grants: no grant on the scope, no observation of the event.
    fn fan_out(
        &self,
        agents: &[RealmAgent],
        events: &[RealmEvent],
        source_name: &str,
        config: &ResolvedRealmConfig,
    ) -> Vec<AgentObservation> {
        let mut out = Vec::new();
        for observer in agents {
            let grants = weaver::read_scopes(&observer.grants);

            for event in events {
                let own = event.source == Some(observer.id);
                if !own && !grants.iter().any(|g| g.matches(&event.scope)) {
                    continue;
                }
                let (role, content) = match event.kind {
                    EventKind::Spoke if own => (ChatRole::Assistant, event.content.clone()),
                    EventKind::Spoke => {
                        (ChatRole::User, config.speech(source_name, &event.content))
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

    pub(super) fn spec_for(
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
            lorebook_ids: Some(agent.lorebooks.clone()),
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
