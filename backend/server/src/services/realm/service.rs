use std::{
    collections::{BTreeMap, HashMap},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use futures_util::{Stream, StreamExt};
use lorealm_storage::{
    Storage,
    models::{
        AGENTS_SCOPE, ActivationState, AgentFilter, AgentGrant, AgentKind, AgentObservation,
        AgentSpec, AgentToolBinding, ChatRole, Conversation, ConversationMessage, Director,
        EventKind, Realm, RealmAgent, RealmCast, RealmConfig, RealmDefinition, RealmEvent,
        RealmOrigin, RealmResourceRef, RealmSummary, RealmVersionInfo, ResolvedRealmConfig,
        Scenario, Snowflake, StateDelta, TurnCommit, UserTool, VersionedSaveMode,
    },
};
use rand::seq::SliceRandom;
use serde_json::json;
use tokio::{sync::RwLock, time::Instant};
use weaver_lang::Value as WeaverValue;

use crate::{
    core::CleanableService,
    errors::{RealmError, TargetType},
    routes::{CreateAgentRequest, CreateScenarioRequest, PlayerTurnRequest, UpdateRealmRequest},
    services::commands::CommandEffect,
    services::{
        character::CharacterService,
        chat::{AgentOutcome, ChatService, FullConversation, InferEvent, TurnAccumulator},
        connection::ConnectionService,
        context::{ContextService, TracedBlock},
        conversation::ConversationService,
        persona::PersonaService,
        plugins::{PluginService, PluginVocabulary},
        realm::{ActionSpec, RealmStreamEvent, TurnResult},
        snowflake::SnowflakeService,
        state::{CapturedState, StatePool, StateService, write_var},
        tools::{EmittedEvent, RealmView, builtin_tool},
    },
    util::context_eval::UsageReport,
};

use super::{
    agent::{
        AgentRegistry, EventDraft, MAIN_CHANNEL, PersistPlan, PlanCx, Production, ReadyTurn,
        TurnPlan, capture, push_emitted_events, push_state_events, speech_channel,
    },
    project::{resolve_scenario_slugs, slug, unique_slug},
    validate::{
        apply_turn_order, realm_resources, validate_director, validate_grants, validate_playable,
        validate_scenario,
    },
    weaver, wtn,
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
    plugins: Arc<PluginService>,
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
        plugins: Arc<PluginService>,
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
            plugins,
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
        Ok(self.owned_realm(realm_id, user_id).await?.definition.config)
    }

    pub async fn set_config(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        config: RealmConfig,
    ) -> Result<RealmConfig, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;
        edit.realm.definition.config = config.clone();
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;
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
            origin: None,
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
        if let Some(director) = req.director {
            if let Some(d) = &director {
                validate_director(d, &edit.realm.definition.agents)?;
            }
            edit.realm.definition.director = director;
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

    /// Roll the head back to a prior version
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
        handle: &str,
        user_id: Snowflake,
    ) -> Result<Vec<AgentGrant>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        let agent = Self::definition_agent(&realm.definition, handle)?;

        Ok(agent.grants.clone())
    }

    /// The agent's action space as currently authored in the preset.
    pub async fn list_actions(
        &self,
        realm_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        let agent = Self::definition_agent(&realm.definition, handle)?.clone();

        self.action_specs(&self.effective_bindings(&agent).await?)
            .await
    }

    pub async fn create_agent(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let parts = self
            .validate_agent(user_id, &req, &edit.realm.definition.agents, None)
            .await?;

        let agent = RealmAgent {
            handle: req.handle,
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            properties: req.properties,
            grants: req.grants,
            tools: req.tools,
            speech_channel: req.speech_channel,
            lorebooks: req.lorebooks,
            widgets: req.widgets,
            script: req.script,
            run_condition: req.run_condition,
        };

        edit.realm.definition.agents.push(agent.clone());
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(agent)
    }

    /// Full-replace edit of a preset agent
    pub async fn update_agent(
        &self,
        realm_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
        req: CreateAgentRequest,
    ) -> Result<RealmAgent, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let parts = self
            .validate_agent(user_id, &req, &edit.realm.definition.agents, Some(handle))
            .await?;

        let slot = edit
            .realm
            .definition
            .agents
            .iter_mut()
            .find(|a| a.handle == handle)
            .ok_or_else(|| RealmError::NoSuchAgent(handle.to_string()))?;

        let agent = RealmAgent {
            handle: req.handle,
            name: req.name,
            kind: req.kind,
            character_id: parts.character_id,
            char_version_id: parts.char_version_id,
            template_id: parts.template_id,
            profile_id: parts.profile_id,
            user_id: parts.user_id,
            persona_id: req.persona_id,
            properties: req.properties,
            grants: req.grants,
            tools: req.tools,
            speech_channel: req.speech_channel,
            lorebooks: req.lorebooks,
            widgets: req.widgets,
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
        handle: &str,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agents = &mut edit.realm.definition.agents;
        let before = agents.len();
        agents.retain(|a| a.handle != handle);
        if agents.len() == before {
            return Err(RealmError::NoSuchAgent(handle.to_string()));
        }
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(())
    }

    pub async fn create_scenario(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        req: CreateScenarioRequest,
    ) -> Result<Scenario, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;
        validate_scenario(&req.name)?;

        let scenario = Scenario {
            slug: unique_slug(
                &slug(&req.name),
                &edit
                    .realm
                    .definition
                    .scenarios
                    .iter()
                    .map(|s| s.slug.clone())
                    .collect::<Vec<_>>(),
            ),
            name: req.name,
            description: req.description,
            opening: req.opening,
        };
        edit.realm.definition.scenarios.push(scenario.clone());
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(scenario)
    }

    /// Full-replace edit of a scenario.
    pub async fn update_scenario(
        &self,
        realm_id: Snowflake,
        scenario_slug: &str,
        user_id: Snowflake,
        req: CreateScenarioRequest,
    ) -> Result<Scenario, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;
        validate_scenario(&req.name)?;

        let slot = edit
            .realm
            .definition
            .scenarios
            .iter_mut()
            .find(|s| s.slug == scenario_slug)
            .ok_or_else(|| RealmError::NoSuchScenario(scenario_slug.to_string()))?;

        // The slug is identity, so a rename keeps it.
        let scenario = Scenario {
            slug: slot.slug.clone(),
            name: req.name,
            description: req.description,
            opening: req.opening,
        };
        *slot = scenario.clone();
        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(scenario)
    }

    pub async fn delete_scenario(
        &self,
        realm_id: Snowflake,
        scenario_slug: &str,
        user_id: Snowflake,
    ) -> Result<(), RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let scenarios = &mut edit.realm.definition.scenarios;
        let before = scenarios.len();
        scenarios.retain(|s| s.slug != scenario_slug);
        if scenarios.len() == before {
            return Err(RealmError::NoSuchScenario(scenario_slug.to_string()));
        }

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(())
    }

    pub async fn set_grants(
        &self,
        realm_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
        grants: Vec<AgentGrant>,
    ) -> Result<(), RealmError> {
        validate_grants(&grants)?;

        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agent = edit
            .realm
            .definition
            .agents
            .iter_mut()
            .find(|a| a.handle == handle)
            .ok_or_else(|| RealmError::NoSuchAgent(handle.to_string()))?;
        agent.grants = grants;

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(())
    }

    /// Rewrite the cast's turn order.
    pub async fn reorder_agents(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        order: Vec<String>,
    ) -> Result<Vec<RealmAgent>, RealmError> {
        let mut edit = self.get_realm_for_edit(realm_id, user_id).await?;

        let agents = &mut edit.realm.definition.agents;
        apply_turn_order(agents, &order)?;

        self.storage.save_realm(&edit.realm, edit.save_mode).await?;

        Ok(edit.realm.definition.agents)
    }

    /// Create a realm by expanding a WTN document.
    pub async fn import(
        &self,
        user_id: Snowflake,
        source: &str,
        inputs: BTreeMap<String, WeaverValue>,
        origin: Option<RealmOrigin>,
    ) -> Result<Realm, RealmError> {
        let doc = wtn::compile(source)?;
        let inputs = self
            .resolve_inputs(user_id, doc.declarations(), inputs)
            .await?;

        let plugins = self.plugins.vocabulary(user_id).await;
        let mut definition = wtn::expand(doc, inputs, plugins.as_ref())?;
        resolve_scenario_slugs(&mut definition.scenarios);
        self.bind_cast(user_id, &mut definition).await?;
        self.validate_definition(user_id, &definition).await?;

        let realm = Realm {
            id: self.snowflake.generate(),
            owner_id: user_id,
            current_version_id: self.snowflake.generate(),
            tags: definition.tags.clone(),
            definition,
            origin,
        };
        self.storage
            .save_realm(&realm, VersionedSaveMode::NewVersion)
            .await?;

        Ok(realm)
    }

    /// The realm as a WTN document.
    pub async fn export(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<String, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        wtn::to_wtn(&realm.definition)
    }

    /// Everything the realm's cast references, derived from the cast itself.
    pub async fn list_resources(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<RealmResourceRef>, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;
        Ok(realm_resources(&realm.definition))
    }

    /// Instantiate a realm: create a conversation bound to it and pin the
    /// cast snapshot from the current version.
    pub async fn start_conversation(
        &self,
        realm_id: Snowflake,
        user_id: Snowflake,
        scenario_slug: Option<String>,
    ) -> Result<FullConversation, RealmError> {
        let realm = self.owned_realm(realm_id, user_id).await?;

        if realm.definition.agents.is_empty() {
            return Err(RealmError::NoAgents(realm_id));
        }
        // The cast may have moved under the director since it was set
        if let Some(director) = &realm.definition.director {
            validate_director(director, &realm.definition.agents)?;
            validate_playable(director)?;
        }
        let scenario = match scenario_slug {
            Some(slug) => Some(
                realm
                    .definition
                    .scenarios
                    .iter()
                    .find(|s| s.slug == slug)
                    .ok_or(RealmError::NoSuchScenario(slug))?
                    .clone(),
            ),
            None => match realm.definition.scenarios.as_slice() {
                [] => None,
                [only] => Some(only.clone()),
                _ => {
                    return Err(RealmError::InvalidScenario(
                        "this realm has multiple scenarios; pick one to start".into(),
                    ));
                }
            },
        };
        let agents = realm.definition.agents;

        let convo = Conversation {
            id: self.snowflake.generate(),
            user_id,
            character_id: None,
            char_version_id: None,
            realm_id: Some(realm_id),
            active_leaf_id: None,
            title: Some(realm.definition.name.clone()),
            author_note: String::new(),
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
                    agents: agents.clone(),
                    director: realm.definition.director.clone(),
                    scenario: scenario.clone(),
                },
            )
            .await?;

        let messages = match self.seed_commit(&convo, &agents, scenario.as_ref()).await? {
            Some(root) => vec![root],
            None => Vec::new(),
        };

        Ok(FullConversation {
            metadata: convo,
            messages,
        })
    }

    /// Commit the playthrough's opening: the cast's declared properties as
    /// the turn-zero state delta, and the scenario's narration if it has one.
    ///
    /// Nothing scripted runs here. State an opening needs beyond the cast's
    /// own properties is a script agent's job, gated on `{{turn:index}} == 0`
    /// and branching on `scenario:`.
    async fn seed_commit(
        &self,
        convo: &Conversation,
        agents: &[RealmAgent],
        scenario: Option<&Scenario>,
    ) -> Result<Option<ConversationMessage>, RealmError> {
        let mut vars: HashMap<String, serde_json::Value> = HashMap::new();
        for agent in agents {
            for (k, v) in &agent.properties {
                write_var(&mut vars, &format!("{}.{k}", agent.handle), v);
            }
        }

        let mut pool = StatePool::new();
        if !vars.is_empty() {
            pool.insert(AGENTS_SCOPE.to_string(), vars);
        }

        let opening = scenario
            .and_then(|s| s.opening.as_deref())
            .map(str::trim)
            .filter(|o| !o.is_empty());
        if pool.is_empty() && opening.is_none() {
            return Ok(None);
        }

        let message = self
            .conversations
            .append_message(
                convo.id,
                ChatRole::System,
                opening.unwrap_or_default().to_string(),
                json!({ "realm_seed": true, "scenario": scenario.map(|s| s.slug.clone()) }),
            )
            .await?;
        if !pool.is_empty() {
            self.state
                .persist(
                    message.id,
                    CapturedState {
                        activation: HashMap::new(),
                        persistent: StateDelta {
                            writes: pool,
                            deletes: HashMap::new(),
                        },
                    },
                )
                .await?;
        }

        // The opening fans out as a sourceless System event on `main`, so
        // every main-granted agent opens with the narration in its feed.
        if let Some(opening) = opening {
            let events = vec![RealmEvent {
                id: self.snowflake.generate(),
                conversation_id: convo.id,
                message_id: message.id,
                seq: 0,
                source: None,
                channel: MAIN_CHANNEL.to_string(),
                kind: EventKind::System,
                content: opening.to_string(),
                payload: json!({}),
            }];
            let config = self.resolve_config(convo).await?;
            let observations = self.fan_out(agents, &events, "", &config);
            self.storage
                .commit_turn(TurnCommit {
                    events,
                    observations,
                })
                .await?;
        }
        Ok(Some(message))
    }

    /// Run the next LLM turn and stream it.
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
            TurnPlan::AwaitPlayer { handle } => {
                let stream = async_stream::try_stream! {
                    yield RealmStreamEvent::AwaitingPlayer { handle };
                };
                return Ok(Box::pin(stream));
            }
            TurnPlan::Ready(ready) => {
                let service = Arc::clone(self);
                let config = config.clone();
                let stream = async_stream::try_stream! {
                    let ReadyTurn { convo, cast, production } = *ready;
                    service.announce_turn(&convo, &production.agent.handle);
                    yield RealmStreamEvent::TurnStarted {
                        handle: production.agent.handle.clone(),
                        name: production.agent.name.clone(),
                    };
                    let TurnResult { message, events } =
                        service.commit(&convo, &cast, &config, production).await?;
                    yield RealmStreamEvent::Committed { message, events };
                };
                // NOTE: If you see an error here, it's a false positive.
                return Ok(Box::pin(stream));
            }
            TurnPlan::Stream(llm) => llm,
        };
        let realm_id = convo
            .realm_id
            .ok_or(RealmError::NotRealmConversation(convo.id))?;
        let realm = self.get_realm(realm_id, user_id).await?;

        self.announce_turn(&convo, &llm.agent.handle);
        let mut open = self
            .chat
            .open_agent_turn(
                &llm.spec,
                &realm,
                &llm.turns,
                &llm.baseline,
                &llm.policy,
                &llm.widget_prompts,
                &llm.widget_renders,
                &convo.author_note,
            )
            .await?;

        let (cancel, guard) = self.chat.begin_cancel(convo_id);

        let service = Arc::clone(self);
        let stream = async_stream::try_stream! {
            let _guard = guard; // the stream owns the cancellation slot
            let llm = *llm;
            yield RealmStreamEvent::TurnStarted {
                handle: llm.agent.handle.clone(),
                name: llm.agent.name.clone(),
            };

            let mut acc = TurnAccumulator::default();
            loop {
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
                    InferEvent::Effect(CommandEffect::Sys { content }) => {
                        tracing::warn!(
                            %content,
                            "ignoring $[sys] from a realm tool body; use $[emit_event]"
                        );
                    }
                    InferEvent::Effect(effect) => {
                        yield RealmStreamEvent::Effect { effect };
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
            let usage = std::mem::take(&mut open.usage);
            let available_tools = std::mem::take(&mut open.available_tools);
            let emitted = llm
                .policy
                .event_sink
                .as_ref()
                .map(|s| std::mem::take(&mut *s.lock().unwrap()))
                .unwrap_or_default();
            let production = service
                .llm_production(&llm.agent, &llm.spec, trace, usage, available_tools, llm.persist, acc.finish(), delta, emitted, activation, open.had_assembly, &config)
                .await?;
            let TurnResult { message, events } =
                service.commit(&llm.convo, &llm.cast, &config, production).await?;
            yield RealmStreamEvent::Committed { message, events };
        };

        // NOTE: If you see an error here, it's a false positive.
        return Ok(Box::pin(stream));
    }

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

        let plugins = self.plugins.vocabulary(user_id).await;
        let stage =
            Stage::new(&cast, &path, &reconstructed.persistent).with_plugins(plugins.clone());
        let agent = stage
            .runnable()?
            .ok_or(RealmError::NoRunnableAgent)?
            .clone();
        let view = stage.view.clone();

        if !matches!(agent.kind, AgentKind::Player) || agent.user_id != Some(user_id) {
            return Err(RealmError::NotYourTurn(agent.handle.clone()));
        }

        let content = req.content.unwrap_or_default();
        if content.trim().is_empty() && req.action.is_none() {
            return Err(RealmError::InvalidTurn(
                "a player turn needs text, an action, or both".into(),
            ));
        }
        self.announce_turn(&convo, &agent.handle);

        let mut delta = StateDelta::default();
        let mut emitted = Vec::new();
        let mut actor_notes = Vec::new();
        let mut action_meta = json!(null);
        let mut acted: Option<(UserTool, AgentToolBinding, serde_json::Value)> = None;
        if let Some(action) = req.action {
            // A player may trigger any tool the agent is bound to, including
            // those bundled by an interactive widget.
            let binding = self
                .effective_bindings(&agent)
                .await?
                .into_iter()
                .find(|b| b.tool_id == action.tool_id)
                .ok_or(RealmError::ToolNotBound(action.tool_id))?;
            let tool = self
                .bound_tool(binding.tool_id)
                .await?
                .ok_or(RealmError::NotFound(TargetType::UserTool, binding.tool_id))?;
            let (output, d, e) = weaver::run_scoped(
                &tool.body,
                &tool.params,
                action.args.clone(),
                &agent,
                &reconstructed.persistent,
                &view,
                plugins.as_ref(),
            )
            .await?;

            actor_notes.push(config.tool_result(&tool.name, &output));
            action_meta = json!({ "tool": tool.name, "input": action.args, "output": output });
            delta = d;
            emitted = e;
            acted = Some((tool, binding, action.args));
        }

        let meta = json!({
            "handle": agent.handle,
            "agent_name": agent.name,
            "action": action_meta,
        });

        let mut events = Vec::new();
        if !content.trim().is_empty() {
            events.push(EventDraft {
                source: agent.handle.clone(),
                channel: speech_channel(&agent),
                kind: EventKind::Spoke,
                content: content.clone(),
                payload: json!({}),
            });
        }
        if let Some((tool, binding, args)) = acted {
            events.push(EventDraft {
                source: agent.handle.clone(),
                channel: binding.event_channel.unwrap_or_default(),
                kind: EventKind::Acted,
                content: config.action(&agent.name, &tool.name),
                payload: json!({ "tool": tool.name, "input": args }),
            });
        }
        push_emitted_events(&agent.handle, emitted, &mut events);
        push_state_events(&agent.handle, &delta, &mut events);

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

    /// The cast snapshot of a playthrough
    pub async fn cast(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmCast, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        Ok(cast)
    }

    /// Whose turn it is at the active leaf once run_conditions are honored
    pub async fn next_agent(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Option<String>, RealmError> {
        let (convo, cast) = self.realm_conversation(convo_id, user_id).await?;

        let path = self.active_path(&convo).await?;
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&ids).await?;

        let plugins = self.plugins.vocabulary(user_id).await;
        Ok(Stage::new(&cast, &path, &world.persistent)
            .with_plugins(plugins)
            .runnable()?
            .map(|a| a.handle.clone()))
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
        handle: &str,
        user_id: Snowflake,
        before: Option<Snowflake>,
    ) -> Result<Vec<AgentObservation>, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        self.cast_agent(&cast, handle)?;
        let observations = self
            .storage
            .list_observations(convo_id, handle, before, super::OBSERVATION_WINDOW)
            .await?;

        Ok(observations)
    }

    /// The agent's grant-filtered view of the pool
    pub async fn agent_state_slice(
        &self,
        convo_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
        at: Option<Snowflake>,
    ) -> Result<StatePool, RealmError> {
        let (convo, cast) = self.realm_conversation(convo_id, user_id).await?;
        let agent = self.cast_agent(&cast, handle)?.clone();

        let path = match at {
            Some(message_id) => {
                self.storage
                    .get_message(message_id)
                    .await?
                    .filter(|m| m.conversation_id == convo.id)
                    .ok_or(RealmError::NotFound(TargetType::Message, message_id))?;
                self.storage.get_path_to(convo.id, message_id).await?
            }
            None => self.active_path(&convo).await?,
        };
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&ids).await?.persistent;

        // Same projection the agent's own prompt sees
        let mut slice = weaver::slice(&world, &weaver::read_scopes_of(&agent));
        weaver::inject_self_alias(&agent, &mut slice);
        Ok(slice)
    }

    /// The agent's action space as pinned in the playthrough's snapshot.
    pub async fn conversation_actions(
        &self,
        convo_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, RealmError> {
        let (_, cast) = self.realm_conversation(convo_id, user_id).await?;
        let agent = self.cast_agent(&cast, handle)?.clone();

        self.action_specs(&self.effective_bindings(&agent).await?)
            .await
    }
}

impl RealmService {
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

    async fn plan_turn(
        &self,
        convo: &Conversation,
        cast: RealmCast,
        target: Option<Snowflake>,
    ) -> Result<TurnPlan, RealmError> {
        let (path, persist) = self.resolve_path(convo, target).await?;

        let path_ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&path_ids).await?;

        let plugins = self.plugins.vocabulary(convo.user_id).await;
        let stage = Stage::new(&cast, &path, &world.persistent).with_plugins(plugins.clone());
        let agent = stage
            .runnable()?
            .ok_or(RealmError::NoRunnableAgent)?
            .clone();
        let view = stage.view.clone();

        self.agents
            .get(&agent.kind)
            .plan(PlanCx {
                service: self,
                convo,
                cast,
                agent,
                view,
                path_ids,
                world,
                persist,
                plugins,
            })
            .await
    }

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

    /// Build the production for a finished LLM turn
    #[allow(clippy::too_many_arguments)]
    async fn llm_production(
        &self,
        agent: &RealmAgent,
        spec: &AgentSpec,
        trace: Vec<TracedBlock>,
        usage: UsageReport,
        available_tools: Vec<String>,
        persist: PersistPlan,
        outcome: AgentOutcome,
        delta: StateDelta,
        emitted: Vec<EmittedEvent>,
        activation: HashMap<Snowflake, ActivationState>,
        had_assembly: bool,
        config: &ResolvedRealmConfig,
    ) -> Result<Production, RealmError> {
        let meta = json!({
            "context": trace,
            "context_usage": usage,
            "reasoning": outcome.reasoning,
            "tool_calls": outcome.tool_calls,
            "widgets": outcome.widgets,
            "handle": agent.handle,
            "agent_name": agent.name,
            "conn_profile": spec.profile_id,
            "tok_id": spec.tokenizer_id,
            "template_id": spec.template_id,
            "available_tools": available_tools,
        });

        let action_scopes = self
            .action_scopes(&self.effective_bindings(agent).await?)
            .await?;

        let mut events = Vec::new();
        if !outcome.spoken.trim().is_empty() {
            events.push(EventDraft {
                source: agent.handle.clone(),
                channel: speech_channel(agent),
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
                source: agent.handle.clone(),
                channel: scope.clone().unwrap_or_default(),
                kind: EventKind::Acted,
                content: config.action(&agent.name, &call.name),
                payload: json!({ "tool": call.name, "input": call.input }),
            });
            actor_notes.push(config.tool_result(&call.name, &call.output.to_string()));
        }

        push_emitted_events(&agent.handle, emitted, &mut events);
        push_state_events(&agent.handle, &delta, &mut events);

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

    /// Persist a produced turn
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

        let events: Vec<RealmEvent> = events
            .into_iter()
            .enumerate()
            .map(|(seq, d)| RealmEvent {
                id: self.snowflake.generate(),
                conversation_id: convo.id,
                message_id: message.id,
                seq: seq as i32,
                source: Some(d.source),
                channel: d.channel,
                kind: d.kind,
                content: d.content,
                payload: d.payload,
            })
            .collect();

        let mut observations = self.fan_out(&cast.agents, &events, &agent.name, config);
        observations.extend(self.actor_notes(convo, message.id, &agent.handle, actor_notes));

        self.storage
            .commit_turn(TurnCommit {
                events: events.clone(),
                observations,
            })
            .await?;

        let plugins = Arc::clone(&self.plugins);
        let (user_id, convo_id, handle) = (convo.user_id, convo.id, agent.handle.clone());
        let (message_id, content) = (message.id, message.content.clone());
        tokio::spawn(async move {
            plugins
                .turn_end(user_id, convo_id, handle, message_id, content)
                .await;
        });

        Ok(TurnResult { message, events })
    }

    /// Tells the user's plugins that `handle`'s turn started, without waiting.
    fn announce_turn(&self, convo: &Conversation, handle: &str) {
        let plugins = Arc::clone(&self.plugins);
        let (user_id, convo_id, handle) = (convo.user_id, convo.id, handle.to_owned());
        tokio::spawn(async move { plugins.turn_start(user_id, convo_id, handle).await });
    }

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

    pub(super) async fn resolve_config(
        &self,
        convo: &Conversation,
    ) -> Result<ResolvedRealmConfig, RealmError> {
        let config = match convo.realm_id {
            Some(id) => self
                .storage
                .get_realm(id)
                .await?
                .map(|r| r.definition.config)
                .unwrap_or_default(),
            None => RealmConfig::default(),
        };
        Ok(ResolvedRealmConfig::resolve(&config))
    }

    fn cast_agent<'a>(
        &self,
        cast: &'a RealmCast,
        handle: &str,
    ) -> Result<&'a RealmAgent, RealmError> {
        cast.agents
            .iter()
            .find(|a| a.handle == handle)
            .ok_or_else(|| RealmError::NoSuchAgent(handle.to_string()))
    }

    fn definition_agent<'a>(
        definition: &'a RealmDefinition,
        handle: &str,
    ) -> Result<&'a RealmAgent, RealmError> {
        definition
            .agents
            .iter()
            .find(|a| a.handle == handle)
            .ok_or_else(|| RealmError::NoSuchAgent(handle.to_string()))
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

    /// Fan events out into per-agent observations
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
                if event.kind == EventKind::StateChanged {
                    continue;
                }
                let own = event.source.as_deref() == Some(observer.handle.as_str());
                if !own && !grants.iter().any(|g| g.matches_channel(&event.channel)) {
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
                    agent_handle: observer.handle.clone(),
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
        agent_handle: &str,
        notes: Vec<String>,
    ) -> Vec<AgentObservation> {
        notes
            .into_iter()
            .map(|content| AgentObservation {
                id: self.snowflake.generate(),
                conversation_id: convo.id,
                message_id,
                agent_handle: agent_handle.to_string(),
                event_id: None,
                role: ChatRole::User,
                content,
            })
            .collect()
    }

    async fn bound_tool(&self, tool_id: Snowflake) -> Result<Option<UserTool>, RealmError> {
        Ok(match builtin_tool(tool_id) {
            Some(tool) => Some(tool),
            None => self.storage.get_tool(tool_id).await?,
        })
    }

    /// The agent's explicit tool bindings
    pub(super) async fn effective_bindings(
        &self,
        agent: &RealmAgent,
    ) -> Result<Vec<AgentToolBinding>, RealmError> {
        let mut bindings = agent.tools.clone();
        for &widget_id in &agent.widgets {
            let Some(widget) = self.storage.get_widget(widget_id).await? else {
                continue;
            };
            for tool_id in widget.tools {
                if !bindings.iter().any(|b| b.tool_id == tool_id) {
                    bindings.push(AgentToolBinding {
                        tool_id,
                        event_channel: None,
                    });
                }
            }
        }
        Ok(bindings)
    }

    /// The directives (and bundled tool ids) of the widgets bound to an agent.
    pub(super) async fn widget_directives(
        &self,
        agent: &RealmAgent,
    ) -> Result<(Vec<String>, Vec<String>, Vec<Snowflake>), RealmError> {
        let mut prompts = Vec::new();
        let mut renders = Vec::new();
        let mut tools = Vec::new();
        for &widget_id in &agent.widgets {
            let Some(widget) = self.storage.get_widget(widget_id).await? else {
                continue;
            };
            if !widget.prompt.trim().is_empty() {
                prompts.push(widget.prompt);
            }
            if !widget.render.trim().is_empty() {
                renders.push(widget.render);
            }
            for id in widget.tools {
                if !tools.contains(&id) {
                    tools.push(id);
                }
            }
        }
        Ok((prompts, renders, tools))
    }

    async fn action_scopes(
        &self,
        bindings: &[AgentToolBinding],
    ) -> Result<HashMap<String, Option<String>>, RealmError> {
        let mut map = HashMap::new();
        for b in bindings {
            if let Some(tool) = self.bound_tool(b.tool_id).await? {
                map.insert(tool.name, b.event_channel.clone());
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
            let Some(tool) = self.bound_tool(binding.tool_id).await? else {
                continue;
            };
            out.push(ActionSpec {
                tool_id: tool.id,
                name: tool.name,
                display_name: tool.display_name,
                description: tool.description,
                params: tool.params,
                event_channel: binding.event_channel.clone(),
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
            agent.handle.clone(),
            "llm agent has no template".to_string(),
        ))?;
        let profile_id = agent.profile_id.ok_or(RealmError::MalformedAgent(
            agent.handle.clone(),
            "llm agent has no connection profile".to_string(),
        ))?;
        let character = match (agent.character_id, agent.char_version_id) {
            (Some(c), Some(v)) => Some((c, v)),
            (None, None) => None,
            _ => {
                return Err(RealmError::MalformedAgent(
                    agent.handle.clone(),
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

/// The handle stamped into a turn's message metadata when it was committed.
fn message_agent_handle(message: &ConversationMessage) -> Option<String> {
    message.metadata.get("handle")?.as_str().map(str::to_string)
}

/// The cast member whose turn produced the last message on the path.
fn last_actor<'a>(
    path: &[ConversationMessage],
    agents: &'a [RealmAgent],
) -> Option<&'a RealmAgent> {
    let last = path.last()?;

    match message_agent_handle(last).and_then(|h| agents.iter().find(|a| a.handle == h)) {
        Some(agent) => Some(agent),
        None if last.metadata.get("realm_seed").is_some() => None,
        None => {
            tracing::warn!(
                message = %last.id,
                "realm message has no resolvable agent; restarting the turn rotation"
            );
            None
        }
    }
}

/// One turn's worth of scheduling context: who is on stage, who just acted,
/// and the world they are acting in.
struct Stage<'a> {
    director: Director,
    agents: &'a [RealmAgent],
    last: Option<&'a RealmAgent>,
    world: &'a StatePool,
    /// The `cast:`/`turn:` mount every body on this turn reads.
    view: RealmView,
    plugins: Option<PluginVocabulary>,
}

impl<'a> Stage<'a> {
    fn new(cast: &'a RealmCast, path: &'a [ConversationMessage], world: &'a StatePool) -> Self {
        let last = last_actor(path, &cast.agents);
        let view = weaver::realm_view(
            &cast.agents,
            &weaver::TurnInfo {
                index: path
                    .iter()
                    .filter(|m| message_agent_handle(m).is_some())
                    .count(),
                last: last.zip(path.last().map(|m| m.content.as_str())),
            },
            cast.scenario.as_ref(),
            world,
        );

        Self {
            director: cast.director.clone().unwrap_or_default(),
            agents: &cast.agents,
            last,
            world,
            view,
            plugins: None,
        }
    }

    /// Lets directors and run conditions call the owner's plugins.
    fn with_plugins(mut self, plugins: Option<PluginVocabulary>) -> Self {
        self.plugins = plugins;
        self
    }

    /// Who the director offers this turn, in preference order.
    fn candidates(&self) -> Result<Vec<&'a RealmAgent>, RealmError> {
        let drawn = |over: &AgentFilter| -> Vec<&'a RealmAgent> {
            self.agents.iter().filter(|a| over.admits(a)).collect()
        };

        Ok(match &self.director {
            Director::RoundRobin { over } => {
                let pool = drawn(over);
                // Resume after whoever acted last
                let start = self
                    .last
                    .and_then(|l| pool.iter().position(|a| a.handle == l.handle))
                    .map_or(0, |i| i + 1);
                (0..pool.len())
                    .map(|k| pool[(start + k) % pool.len()])
                    .collect()
            }
            Director::Random { over } => {
                let mut pool = drawn(over);
                pool.shuffle(&mut rand::rng());
                pool
            }
            Director::Script { source } => {
                weaver::run_director(source, self.world, &self.view, self.plugins.as_ref())?
                    .into_iter()
                    .map(|handle| {
                        self.agents.iter().find(|a| a.handle == handle).ok_or(
                            RealmError::DirectorFailed(format!(
                                "no cast member holds the handle '{handle}'"
                            )),
                        )
                    })
                    .collect::<Result<_, _>>()?
            }
        })
    }

    /// The first candidate its run condition lets through.
    fn runnable(&self) -> Result<Option<&'a RealmAgent>, RealmError> {
        Ok(self.candidates()?.into_iter().find(|a| {
            weaver::run_condition_passes(a, self.world, &self.view, self.plugins.as_ref())
        }))
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

#[cfg(test)]
mod tests {
    use super::*;
    use lorealm_storage::models::AgentAccess;

    fn agent(slot: i64, name: &str, kind: AgentKind, run_condition: Option<&str>) -> RealmAgent {
        RealmAgent {
            handle: format!("a{slot}"),
            name: name.into(),
            kind,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            user_id: None,
            persona_id: None,
            properties: HashMap::new(),
            grants: vec![AgentGrant {
                channel: "state".into(),
                access: AgentAccess::ReadWrite,
            }],
            tools: Vec::new(),
            speech_channel: None,
            lorebooks: Vec::new(),
            widgets: Vec::new(),
            script: None,
            run_condition: run_condition.map(Into::into),
        }
    }

    fn cast() -> Vec<RealmAgent> {
        vec![
            agent(0, "Player", AgentKind::Player, None),
            agent(1, "Script", AgentKind::Script, None),
            agent(
                2,
                "Agent #1",
                AgentKind::Llm,
                Some("{{state:index}} == 0 || {{state:index}} == 1"),
            ),
            agent(
                3,
                "Agent #2",
                AgentKind::Llm,
                Some("{{state:index}} == 0 || {{state:index}} == 2"),
            ),
        ]
    }

    fn world(index: i64) -> StatePool {
        let mut w = StatePool::new();
        w.entry("state".into())
            .or_default()
            .insert("index".into(), json!(index));
        w
    }

    fn message(id: i64, agent: Option<&RealmAgent>) -> ConversationMessage {
        ConversationMessage {
            id: Snowflake(id),
            conversation_id: Snowflake(100),
            parent: None,
            role: ChatRole::Assistant,
            content: String::new(),
            order: id as i32,
            metadata: match agent {
                Some(a) => json!({ "handle": a.handle, "agent_name": a.name }),
                None => json!({}),
            },
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    fn casting(agents: Vec<RealmAgent>, director: Option<Director>) -> RealmCast {
        RealmCast {
            realm_id: Snowflake(99),
            version_id: None,
            agents,
            director,
            scenario: None,
        }
    }

    /// Who acts after `last_slot` (`None` = nothing has acted), with the world
    /// at `index`.
    fn next_after(
        agents: &[RealmAgent],
        director: Option<Director>,
        last_slot: Option<usize>,
        index: i64,
    ) -> Option<String> {
        let path: Vec<ConversationMessage> = last_slot
            .map(|slot| vec![message(0, Some(&agents[slot]))])
            .unwrap_or_default();
        let cast = casting(agents.to_vec(), director);
        let world = world(index);

        Stage::new(&cast, &path, &world)
            .runnable()
            .unwrap()
            .map(|a| a.name.clone())
    }

    #[test]
    fn an_empty_path_starts_at_the_top_of_the_cast() {
        assert_eq!(
            next_after(&cast(), None, None, 0).as_deref(),
            Some("Player")
        );
    }

    #[test]
    fn the_rotation_follows_the_last_actor_not_the_path_length() {
        let cast = cast();
        let path: Vec<ConversationMessage> = [0, 1, 2, 3, 0, 1, 2, 0]
            .iter()
            .enumerate()
            .map(|(i, slot)| message(i as i64, Some(&cast[*slot])))
            .collect();

        assert_eq!(path.len(), 8);
        let casting = casting(cast.clone(), None);
        let world = world(1);
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .runnable()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Script"),
        );
    }

    #[test]
    fn an_unattributable_last_turn_restarts_the_rotation() {
        let path = [message(0, None)];
        let casting = casting(cast(), None);
        let world = world(0);
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .runnable()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Player"),
        );
    }

    #[test]
    fn a_gated_agent_is_skipped_without_stalling_the_rotation() {
        let cast = cast();
        // After the script agent, #1 is ungated at index 1 and runs.
        assert_eq!(
            next_after(&cast, None, Some(1), 1).as_deref(),
            Some("Agent #1")
        );
        // After #1, #2 is gated at index 1, so the rotation wraps to the player.
        assert_eq!(
            next_after(&cast, None, Some(2), 1).as_deref(),
            Some("Player")
        );
    }

    #[test]
    fn skips_keep_the_group_chat_cycle_aligned() {
        let cast = cast();
        let next = |last_slot: usize, index: i64| next_after(&cast, None, Some(last_slot), index);

        // index 0: nobody is gated, so the whole cast speaks in order.
        assert_eq!(next(0, 0).as_deref(), Some("Script"));
        assert_eq!(next(1, 0).as_deref(), Some("Agent #1"));
        assert_eq!(next(2, 0).as_deref(), Some("Agent #2"));
        assert_eq!(next(3, 0).as_deref(), Some("Player"));

        // index 2: Agent #1 sits out and Agent #2 speaks in its place.
        assert_eq!(next(1, 2).as_deref(), Some("Agent #2"));
        assert_eq!(next(3, 2).as_deref(), Some("Player"));
    }

    #[test]
    fn no_runnable_agent_when_every_condition_is_false() {
        let llms = cast()[2..].to_vec();
        assert_eq!(next_after(&llms, None, None, 9), None);
    }

    #[test]
    fn a_filter_narrows_the_rotation_to_the_agents_it_admits() {
        let cast = cast();
        let llm_only = || {
            Some(Director::RoundRobin {
                over: AgentFilter::Kind {
                    kind: AgentKind::Llm,
                },
            })
        };

        assert_eq!(
            next_after(&cast, llm_only(), Some(2), 0).as_deref(),
            Some("Agent #2")
        );
        // Wrapping from the end of the pool comes back to its start.
        assert_eq!(
            next_after(&cast, llm_only(), Some(3), 0).as_deref(),
            Some("Agent #1")
        );
        // A last actor outside the pool resumes at the next member after it.
        assert_eq!(
            next_after(&cast, llm_only(), Some(1), 0).as_deref(),
            Some("Agent #1")
        );
    }

    #[test]
    fn a_script_director_names_who_acts() {
        let cast = cast();
        let pick = |source: &str| {
            next_after(
                &cast,
                Some(Director::Script {
                    source: source.into(),
                }),
                Some(0),
                0,
            )
        };

        assert_eq!(pick("a3").as_deref(), Some("Agent #2"));
        // An array is a preference order, and the first entry wins when its
        // run condition passes.
        assert_eq!(
            pick(r#"{# return ["a2", "a3"] #}"#).as_deref(),
            Some("Agent #1")
        );
        // Naming nobody is a legal answer: no one acts.
        assert_eq!(pick(""), None);
    }

    #[test]
    fn a_script_director_that_names_a_stranger_fails_loudly() {
        let casting = casting(
            cast(),
            Some(Director::Script {
                source: "nobody".into(),
            }),
        );
        let world = world(0);
        let err = Stage::new(&casting, &[], &world).runnable().unwrap_err();
        assert!(matches!(err, RealmError::DirectorFailed(_)), "{err}");
    }

    #[test]
    fn a_script_director_reads_the_realm_it_schedules() {
        let cast = cast();
        let source = r#"@[array.at(
            items: {{cast:llm.handles}},
            index: @[math.mod(a: {{turn:index}}, b: {{cast:llm.count}})]
        )]"#;

        let path: Vec<ConversationMessage> = vec![message(0, Some(&cast[0]))];
        let casting = casting(
            cast,
            Some(Director::Script {
                source: source.into(),
            }),
        );
        let world = world(0);
        // One turn on the path, two LLMs: 1 % 2 picks the second.
        assert_eq!(
            Stage::new(&casting, &path, &world)
                .runnable()
                .unwrap()
                .map(|a| a.name.as_str()),
            Some("Agent #2"),
        );
    }
}
