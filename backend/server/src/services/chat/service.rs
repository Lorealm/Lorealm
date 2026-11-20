use std::{
    borrow::Cow,
    collections::HashMap,
    pin::Pin,
    sync::{Arc, OnceLock, Weak},
};

use chrono::Utc;
use dashmap::DashMap;
use futures_util::{Stream, StreamExt};
use lorealm_providers::{
    AnthropicOptions, AnthropicProvider, BlockAccumulator, Client, ContentBlock, LlmProvider,
    OpenAiOptions, OpenAiProvider, OpenRouterOptions, OpenRouterProvider, VertexAuth,
    VertexOptions, VertexProvider,
};
use lorealm_storage::{
    Storage,
    models::{
        AGENTS_SCOPE, AgentAccess, AgentGrant, AgentKind, AgentSpec, ChatRole, Conversation,
        ConversationMessage, ConversationMetadata, RealmAgent, RealmCast, RealmConfig, Snowflake,
        StateDelta, WidgetPlacement,
    },
};
use serde_json::json;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::{ChatError, RealmError, TargetType},
    services::{
        character::CharacterService,
        chat::{
            ActionSpec, ActiveWidgets, CancelGuard, FullConversation, Prepared, PromptMessage,
            perceive::perceive,
            stage::Stage,
            weaver,
            widgets::{
                AttachmentSpec, InferEvent, ToolCallRecord, WidgetInstance, WidgetRenderer,
                WidgetSink, render_widgets,
            },
        },
        commands::{CommandEffect, EffectSink, GenerateBridge, Generation},
        connection::ConnectionService,
        context_template::{
            ContextTemplate, PromptRole, PromptTurn, ResolveContext, SourceRegistry, assemble,
        },
        conversation::{ConversationService, EditMode},
        persona::PersonaService,
        plugins::PluginService,
        schedule::{Schedule, stages::PrivacyStage},
        snowflake::SnowflakeService,
        state::{CapturedState, ReconstructedState, StatePool, StateService, write_var},
        tokenizer::TokenizerService,
        tools::{
            ATTACHMENT_TOOL_IDS, CompiledUserTool, RealmVariables, ToolPolicy, ToolSignature,
            UserToolService, builtin_tool,
        },
        user_widgets::builtin_widgets,
        variables::{VariableContext, VariableRegistry},
        weaver::{WeaverHost, WeaverProfile},
    },
    util::context_eval::{ContextEvalResult, ContextEvaluator, TokenCounter},
};

pub struct ChatService {
    pub(super) storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    pub(super) conversations: Arc<ConversationService>,
    pub(super) characters: Arc<CharacterService>,
    pub(super) personas: Arc<PersonaService>,
    pub(super) state: Arc<StateService>,
    connection: Arc<ConnectionService>,
    tools: Arc<UserToolService>,
    tokenizers: Arc<TokenizerService>,
    sources: Arc<SourceRegistry>,
    variables: Arc<VariableRegistry>,
    pub(super) plugins: Arc<PluginService>,
    pub(super) schedule: Arc<Schedule>,
    cancellations: Arc<DashMap<Snowflake, CancellationToken>>,
    /// A handle back to itself, for the `$[generate]` a tool body may call.
    ///
    /// The generate commands hold an owned `'static` handle, which `&self`
    /// cannot provide, and a strong reference here would be a cycle.
    me: OnceLock<Weak<ChatService>>,
}

impl ChatService {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        conversations: Arc<ConversationService>,
        characters: Arc<CharacterService>,
        state: Arc<StateService>,
        personas: Arc<PersonaService>,
        connection: Arc<ConnectionService>,
        tools: Arc<UserToolService>,
        tokenizers: Arc<TokenizerService>,
        sources: Arc<SourceRegistry>,
        variables: Arc<VariableRegistry>,
        plugins: Arc<PluginService>,
    ) -> Self {
        Self {
            storage,
            snowflake,
            conversations,
            characters,
            state,
            personas,
            connection,
            tools,
            tokenizers,
            sources,
            variables,
            plugins,
            schedule: Arc::new(Schedule::new(vec![Box::new(PrivacyStage)])),
            cancellations: Arc::new(DashMap::new()),
            me: OnceLock::new(),
        }
    }

    /// Hand the service a weak handle to its own `Arc`. Called once, right
    /// after construction; see [`ChatService::me`].
    pub fn set_self(&self, me: Weak<ChatService>) {
        let _ = self.me.set(me);
    }

    fn generation(&self) -> Option<Arc<dyn Generation>> {
        Some(self.me.get()?.upgrade()? as Arc<dyn Generation>)
    }

    pub async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, ChatError> {
        let convos = self.storage.list_conversations(user_id).await?;

        Ok(convos)
    }

    pub async fn list_messages(
        &self,
        conversation_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        let _convo = self.get_conversation(conversation_id, user_id).await?;
        let convos = self.storage.list_messages(conversation_id).await?;

        Ok(convos)
    }

    pub async fn get_message(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<ConversationMessage, ChatError> {
        let _convo = self.get_conversation(conversation_id, user_id).await?;
        let message = self
            .storage
            .get_message(message_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Message, message_id))?;
        if message.conversation_id != conversation_id {
            return Err(ChatError::NotFound(TargetType::Message, message_id));
        }
        Ok(message)
    }

    pub async fn get_path_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        let _convo = self.get_conversation(conversation_id, user_id).await?;
        let messages = self
            .storage
            .get_path_to(conversation_id, message_id)
            .await?;

        Ok(messages)
    }

    pub(super) async fn get_conversation(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Conversation, ChatError> {
        let convo = self
            .storage
            .get_conversation(convo_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Conversation, convo_id))?;

        if convo.user_id != user_id {
            return Err(ChatError::NotOwned(TargetType::Conversation, convo_id));
        }

        Ok(convo)
    }

    pub async fn get_full_conversation(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        full_msg_tree: bool,
    ) -> Result<FullConversation, ChatError> {
        let convo = self.get_conversation(convo_id, user_id).await?;
        let messages = if full_msg_tree {
            self.storage.list_messages(convo_id).await?
        } else {
            self.storage.get_active_path(convo_id).await?
        };
        Ok(FullConversation {
            metadata: convo,
            messages,
        })
    }

    /// Start a chat with a character
    ///
    /// An 1-on-1 conversation is a realm with two actors: the user and the character
    /// they're speaking with.
    pub async fn start_with_character(
        &self,
        user_id: Snowflake,
        character_id: Snowflake,
        version_id: Option<Snowflake>,
        greeting_id: Snowflake,
        author_note: String,
    ) -> Result<FullConversation, ChatError> {
        let version_id = match version_id {
            Some(id) => id,
            None => self.characters.get_latest_version_id(character_id).await?,
        };
        let character = self
            .characters
            .load_version(character_id, version_id, user_id)
            .await?;
        let greeting = character
            .definition
            .greetings
            .iter()
            .find(|g| g.id == greeting_id)
            .ok_or(ChatError::NotFound(TargetType::Greeting, greeting_id))?;
        let active_persona = self.state.get_active_persona(user_id).await?;
        let you = self
            .personas
            .get_persona_for_character(user_id, character_id, active_persona.id)
            .await?;

        let member = |handle: &str, name: String, kind| RealmAgent {
            handle: handle.into(),
            name,
            kind,
            character_id: None,
            char_version_id: None,
            template_id: None,
            profile_id: None,
            persona_id: None,
            user_id: None,
            properties: HashMap::new(),
            grants: vec![AgentGrant {
                channel: "*".into(),
                access: AgentAccess::ReadWrite,
            }],
            tools: Vec::new(),
            lorebooks: Vec::new(),
            widgets: Vec::new(),
            script: None,
            run_condition: None,
        };
        let cast = RealmCast {
            version_id: None,
            agents: vec![
                RealmAgent {
                    user_id: Some(user_id),
                    ..member(
                        "user",
                        you.display_name.unwrap_or(you.name),
                        AgentKind::Player,
                    )
                },
                RealmAgent {
                    character_id: Some(character_id),
                    char_version_id: Some(version_id),
                    ..member("char", character.definition.name.clone(), AgentKind::Llm)
                },
            ],
            director: None,
            scenario: None,
            config: RealmConfig {
                speech_format: Some("{content}".into()),
                ..Default::default()
            },
            modules: Vec::new(),
        };

        let convo = self
            .create_conversation(
                user_id,
                Some((character_id, version_id)),
                None,
                None,
                author_note,
                &cast,
            )
            .await?;
        let opening = self
            .conversations
            .append_message(
                convo.id,
                ChatRole::Assistant,
                greeting.text.clone(),
                json!({ "handle": "char", "agent_name": character.definition.name }),
            )
            .await?;

        Ok(FullConversation {
            metadata: Conversation {
                active_leaf_id: Some(opening.id),
                ..convo
            },
            messages: vec![opening],
        })
    }

    /// Start a playthrough of a realm, with the cast of its current version.
    pub async fn start_with_realm(
        &self,
        user_id: Snowflake,
        realm_id: Snowflake,
        scenario_slug: Option<String>,
    ) -> Result<FullConversation, ChatError> {
        let realm = self
            .storage
            .get_realm(realm_id)
            .await?
            .filter(|r| r.owner_id == user_id)
            .ok_or(ChatError::NotFound(TargetType::Realm, realm_id))?;
        let definition = realm.definition;

        if definition.agents.is_empty() {
            return Err(RealmError::NoAgents(realm_id).into());
        }
        let scenario = match scenario_slug {
            Some(slug) => Some(
                definition
                    .scenarios
                    .iter()
                    .find(|s| s.slug == slug)
                    .ok_or(RealmError::NoSuchScenario(slug))?
                    .clone(),
            ),
            None => match definition.scenarios.as_slice() {
                [] => None,
                [only] => Some(only.clone()),
                _ => {
                    return Err(RealmError::InvalidScenario(
                        "this realm has multiple scenarios; pick one to start".into(),
                    )
                    .into());
                }
            },
        };

        let mut seeded: HashMap<String, serde_json::Value> = HashMap::new();
        for agent in &definition.agents {
            for (key, value) in &agent.properties {
                write_var(&mut seeded, &format!("{}.{key}", agent.handle), value);
            }
        }
        let opening = scenario
            .as_ref()
            .and_then(|s| s.opening.as_deref())
            .map(str::trim)
            .unwrap_or_default()
            .to_string();

        let cast = RealmCast {
            version_id: Some(realm.current_version_id),
            agents: definition.agents,
            director: definition.director,
            scenario,
            config: definition.config,
            modules: definition.modules,
        };
        let mut convo = self
            .create_conversation(
                user_id,
                None,
                Some(realm_id),
                Some(definition.name),
                String::new(),
                &cast,
            )
            .await?;

        let mut messages = Vec::new();
        if !seeded.is_empty() || !opening.is_empty() {
            let message = self
                .conversations
                .append_message(convo.id, ChatRole::System, opening, json!({}))
                .await?;
            if !seeded.is_empty() {
                self.state
                    .persist(
                        message.id,
                        CapturedState {
                            activation: HashMap::new(),
                            persistent: StateDelta {
                                writes: HashMap::from([(AGENTS_SCOPE.to_string(), seeded)]),
                                deletes: HashMap::new(),
                            },
                        },
                    )
                    .await?;
            }
            convo.active_leaf_id = Some(message.id);
            messages.push(message);
        }

        Ok(FullConversation {
            metadata: convo,
            messages,
        })
    }

    async fn create_conversation(
        &self,
        user_id: Snowflake,
        character: Option<(Snowflake, Snowflake)>,
        realm_id: Option<Snowflake>,
        title: Option<String>,
        author_note: String,
        cast: &RealmCast,
    ) -> Result<Conversation, ChatError> {
        let convo = Conversation {
            id: self.snowflake.generate(),
            user_id,
            character_id: character.map(|(id, _)| id),
            char_version_id: character.map(|(_, version)| version),
            realm_id,
            active_leaf_id: None,
            title,
            author_note,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        self.storage.create_conversation(&convo).await?;
        self.storage.put_cast(convo.id, cast).await?;
        Ok(convo)
    }

    /// The cast a conversation plays with.
    pub(super) async fn cast_of(&self, convo_id: Snowflake) -> Result<RealmCast, ChatError> {
        self.storage
            .get_cast(convo_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Conversation, convo_id))
    }

    pub async fn cast(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<RealmCast, ChatError> {
        self.get_conversation(convo_id, user_id).await?;
        self.cast_of(convo_id).await
    }

    /// Whose turn it is at the active leaf.
    pub async fn next_agent(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Option<String>, ChatError> {
        let cast = self.cast(convo_id, user_id).await?;
        self.whose_turn(convo_id, &cast).await
    }

    async fn cast_member(
        &self,
        convo_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
    ) -> Result<RealmAgent, ChatError> {
        let cast = self.cast(convo_id, user_id).await?;
        Ok(cast
            .agents
            .into_iter()
            .find(|a| a.handle == handle)
            .ok_or_else(|| RealmError::NoSuchAgent(handle.to_string()))?)
    }

    pub async fn agent_state(
        &self,
        convo_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
        at: Option<Snowflake>,
    ) -> Result<StatePool, ChatError> {
        let agent = self.cast_member(convo_id, handle, user_id).await?;
        let path = match at {
            Some(message_id) => self.get_path_to(convo_id, message_id, user_id).await?,
            None => self.storage.get_active_path(convo_id).await?,
        };
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let world = self.state.reconstruct(&ids).await?.persistent;

        // Same projection the agent's own prompt sees
        let mut slice = weaver::slice(&world, &weaver::read_scopes_of(&agent));
        weaver::inject_self_alias(&agent, &mut slice);
        Ok(slice)
    }

    /// The tools an agent is bound to.
    pub async fn agent_actions(
        &self,
        convo_id: Snowflake,
        handle: &str,
        user_id: Snowflake,
    ) -> Result<Vec<ActionSpec>, ChatError> {
        let agent = self.cast_member(convo_id, handle, user_id).await?;

        let mut actions = Vec::new();
        for binding in self.tool_bindings(&agent).await? {
            let Some(tool) = self.bound_tool(binding.tool_id).await? else {
                continue;
            };
            actions.push(ActionSpec {
                tool_id: tool.id,
                name: tool.name,
                display_name: tool.display_name,
                description: tool.description,
                params: tool.params,
                event_channel: binding.event_channel,
            });
        }
        Ok(actions)
    }

    /// Edit a message in place. No new branch; the continuation below it stays.
    pub async fn edit_message(
        &self,
        convo_id: Snowflake,
        msg_id: Snowflake,
        user_id: Snowflake,
        content: String,
    ) -> Result<ConversationMessage, ChatError> {
        self.get_conversation(convo_id, user_id).await?;

        Ok(self
            .conversations
            .edit_message(convo_id, msg_id, content, EditMode::InPlace)
            .await?)
    }

    /// Move the active path so it runs through `msg_id`
    pub async fn activate_message(
        &self,
        convo_id: Snowflake,
        msg_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        self.get_conversation(convo_id, user_id).await?;

        Ok(self.conversations.switch_to(convo_id, msg_id).await?)
    }

    /// Delete a message and its entire subtree. If the active leaf was inside it,
    /// the leaf is repointed to the fork
    pub async fn delete_message(
        &self,
        convo_id: Snowflake,
        msg_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), ChatError> {
        self.get_conversation(convo_id, user_id).await?;

        Ok(self.conversations.delete_subtree(convo_id, msg_id).await?)
    }

    /// Delete an entire conversation
    pub async fn delete_conversation(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
    ) -> Result<(), ChatError> {
        self.get_conversation(convo_id, user_id).await?;

        self.storage.delete_conversation(convo_id).await?;
        Ok(())
    }

    pub async fn set_author_note(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        note: String,
    ) -> Result<(), ChatError> {
        self.get_conversation(convo_id, user_id).await?;

        self.storage.set_author_note(convo_id, &note).await?;
        Ok(())
    }

    /// Message exists AND belongs to this conversation.
    pub(super) async fn owned_message(
        &self,
        msg_id: Snowflake,
        convo_id: Snowflake,
    ) -> Result<ConversationMessage, ChatError> {
        let msg = self
            .storage
            .get_message(msg_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Message, msg_id))?;
        if msg.conversation_id != convo_id {
            return Err(ChatError::NotOwned(TargetType::Message, msg_id));
        }
        Ok(msg)
    }

    /// The widgets in play for a turn
    pub(super) async fn active_widgets(
        &self,
        user_id: Snowflake,
        attached: &[Snowflake],
    ) -> Result<ActiveWidgets, ChatError> {
        let mut widgets = self.storage.list_enabled_widgets(user_id).await?;
        let builtins: Vec<_> = builtin_widgets()
            .iter()
            .filter(|b| widgets.iter().all(|w| w.name != b.name))
            .cloned()
            .collect();
        widgets.extend(builtins);
        for &id in attached {
            if widgets.iter().any(|w| w.id == id) {
                continue;
            }
            // A widget deleted out from under the character just drops out.
            let Some(widget) = self.storage.get_widget(id).await? else {
                tracing::warn!(
                    "A character-bound widget was to be loaded, but could not be found: {id}"
                );
                continue;
            };
            if widget.user_id != user_id {
                continue;
            }
            widgets.push(widget);
        }

        let mut prompts = Vec::new();
        let mut renders = Vec::new();
        let mut tools = Vec::new();
        let mut catalog = Vec::new();
        for w in widgets {
            if w.placement == WidgetPlacement::Attachment {
                catalog.push(AttachmentSpec::from(&w));
                continue;
            }
            if !w.prompt.trim().is_empty() {
                prompts.push(w.prompt);
            }
            if !w.render.trim().is_empty() {
                renders.push(w.render);
            }
            for id in w.tools {
                if !tools.contains(&id) {
                    tools.push(id);
                }
            }
        }
        Ok(ActiveWidgets {
            prompts,
            renders,
            tools,
            attachments: catalog,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn assemble_context(
        &self,
        spec: &AgentSpec,
        cast: &RealmCast,
        agent: &RealmAgent,
        view: &RealmVariables,
        conversation: Option<Snowflake>,
        thread: &[PromptTurn],
        baseline: &ReconstructedState,
        widget_prompts: &[String],
        widget_renders: &[String],
        author_note: &str,
    ) -> Result<Prepared, ChatError> {
        let character = match spec.character {
            Some((cid, cvid)) => Some(self.characters.load_version(cid, cvid, spec.owner).await?),
            None => None,
        };
        let persona = match spec.persona_id {
            Some(id) => Some(self.personas.get_persona(id, spec.owner).await?),
            None => None,
        };
        let template = {
            let db = self
                .storage
                .get_template_owned(spec.template_id, spec.owner)
                .await?
                .ok_or(ChatError::NotFound(
                    TargetType::ContextTemplate,
                    spec.template_id,
                ))?;
            ContextTemplate::from_db(db)?
        };
        let tokenizer = self
            .tokenizers
            .load_kitoken_tokenizer(spec.tokenizer_id, spec.owner)
            .await?;

        let plugins = self
            .plugins
            .vocabulary_with(spec.owner, &cast.modules)
            .await;
        let widget_renders = render_widgets(widget_renders, &baseline.persistent, plugins.as_ref());
        let sources = match &plugins {
            Some(plugins) => {
                let mut sources = (*self.sources).clone();
                plugins.sources().for_each(|s| sources.register(s));
                Cow::Owned(sources)
            }
            None => Cow::Borrowed(&*self.sources),
        };

        let captured;
        let trace = {
            let ctx = ResolveContext {
                character: character.as_ref(),
                conversation,
                messages: thread,
                persona: persona.as_ref(),
                user_id: spec.owner,
                state: baseline,
                lorebook_state: Mutex::new(None),
                lorebook_ids: spec.lorebook_ids.as_deref(),
                tokenizer: &tokenizer,
                variables: self.variables.snapshot(&VariableContext {
                    character: character.as_ref(),
                    persona: persona.as_ref(),
                    cast: Some(cast),
                }),
                widget_prompts,
                widget_renders: &widget_renders,
                author_note,
                plugins,
                actor: Some(weaver::actor_of(agent)),
                realm: Some(view.clone()),
            };
            let trace = assemble(&template, &sources, &ctx).await?;
            captured = ctx.lorebook_state.lock().await.take();
            trace
        };

        let budget = self
            .connection
            .get_profile(spec.owner, spec.profile_id)
            .await?
            .token_budget;

        let mut evaluator =
            ContextEvaluator::builder(Arc::clone(&tokenizer) as Arc<dyn TokenCounter>);
        if budget > 0 {
            evaluator = evaluator.budget(budget as u32);
        }
        let ContextEvalResult {
            messages,
            trace,
            usage,
        } = evaluator.build().evaluate(trace)?;
        let messages = self
            .plugins
            .prompt_build(spec.owner, conversation, messages)
            .await;

        Ok(Prepared {
            prompt: messages,
            trace,
            captured,
            usage,
        })
    }

    /// Like [`Self::begin_cancel`], but refuses when the conversation is
    /// already generating rather than displacing its token.
    pub(crate) fn try_begin_cancel(
        &self,
        convo_id: Snowflake,
    ) -> Option<(CancellationToken, CancelGuard)> {
        let cancel = CancellationToken::new();
        match self.cancellations.entry(convo_id) {
            dashmap::Entry::Occupied(_) => None,
            dashmap::Entry::Vacant(slot) => {
                slot.insert(cancel.clone());
                Some((
                    cancel,
                    CancelGuard {
                        map: Arc::clone(&self.cancellations),
                        key: convo_id,
                    },
                ))
            }
        }
    }

    pub(crate) fn begin_cancel(&self, convo_id: Snowflake) -> (CancellationToken, CancelGuard) {
        let cancel = CancellationToken::new();
        self.cancellations.insert(convo_id, cancel.clone());
        let guard = CancelGuard {
            map: Arc::clone(&self.cancellations),
            key: convo_id,
        };
        (cancel, guard)
    }

    pub async fn cancel(&self, convo_id: Snowflake, user_id: Snowflake) -> Result<bool, ChatError> {
        self.get_conversation(convo_id, user_id).await?;
        Ok(match self.cancellations.get(&convo_id) {
            Some(tok) => {
                tok.cancel();
                true
            }
            None => false,
        })
    }

    /// Token + widget stream from the model.
    pub(super) async fn infer(
        &self,
        owner: Snowflake,
        profile_id: Snowflake,
        prompt: Vec<super::PromptMessage>,
        state: Arc<StdMutex<StatePool>>,
        policy: &ToolPolicy,
    ) -> Result<
        (
            Vec<String>,
            Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>,
        ),
        ChatError,
    > {
        let profile = self.connection.get_profile(owner, profile_id).await?;
        let connection = self
            .connection
            .resolve_connection(owner, profile.id)
            .await?;

        let api_key = connection.api_key;
        let Some(model) = connection.profile.model else {
            return Err(ChatError::Connection("no model".into()));
        };

        let common: lorealm_providers::CommonParams =
            serde_json::from_value(connection.profile.common_params).map_err(|e| {
                ChatError::Connection(format!("Failed to deserialize common params: {e}"))
            })?;

        // Map your custom payload directly to what the provider client expects
        let messages: Vec<lorealm_providers::Message> = prompt
            .into_iter()
            .map(|m| lorealm_providers::Message {
                role: prompt_role_to_provider_role(m.role),
                content: vec![ContentBlock::Text { text: m.content }],
            })
            .collect();

        let mut registry = lorealm_providers::ToolRegistry::new();

        // Plugin callables are only reachable from tool bodies.
        let offers_tools = policy.tool_ids.as_ref().is_none_or(|ids| !ids.is_empty())
            || !policy.attachments.is_empty()
            || !policy.extra_tool_ids.is_empty();
        let plugins = match offers_tools {
            true => self.plugins.vocabulary_with(owner, &policy.modules).await,
            false => None,
        };

        let mut host = WeaverHost::builder()
            .profile(WeaverProfile::ChatTurn)
            .attachments(policy.attachments.clone())
            .pool(Arc::clone(&state))
            .plugins(plugins);

        // Realm turns hand over a sink so the events a tool emits are read
        // back by the realm that scheduled the turn.
        if let Some(sink) = &policy.event_sink {
            host = host.events(Arc::clone(sink));
        }

        // A tool body's `$[generate]` needs a handle back to this service and
        // a conversation to assemble context from.
        if let (Some(chat), Some(conversation)) = (self.generation(), policy.conversation) {
            host = host.generation(GenerateBridge {
                handle: tokio::runtime::Handle::current(),
                chat,
                state: Arc::clone(&self.state),
                owner,
                conversation,
                cancel: policy.cancel.clone().unwrap_or_default(),
            });
        }

        let host = host.build();
        let module_values = host.module_exports();
        let (weaver_registry, sinks, state) = host.into_shared();
        let widget_sink = sinks.widgets;
        let effect_sink = sinks.effects;

        let mut rows = match &policy.tool_ids {
            None => self.tools.list_enabled(owner).await?,
            Some(ids) => {
                let mut rows = Vec::with_capacity(ids.len());
                for id in ids {
                    rows.push(match builtin_tool(*id) {
                        Some(tool) => tool,
                        None => self.tools.get_tool(*id, owner).await?,
                    });
                }
                rows
            }
        };

        if !policy.attachments.is_empty() {
            for id in ATTACHMENT_TOOL_IDS {
                if !rows.iter().any(|r| r.id == id) {
                    rows.extend(builtin_tool(id));
                }
            }
        }

        // Widgets and characters bundle their tools regardless of the selection above.
        for id in &policy.extra_tool_ids {
            if rows.iter().any(|r| r.id == *id) {
                continue;
            }
            rows.push(match builtin_tool(*id) {
                Some(tool) => tool,
                None => match self.tools.get_tool(*id, owner).await {
                    Ok(tool) => tool,
                    // Deleted (or no longer the owner's) since it was attached:
                    // the chat goes on without it.
                    Err(
                        crate::errors::UserToolError::NotFound(..)
                        | crate::errors::UserToolError::NotOwned,
                    ) => {
                        tracing::warn!("skipping attached tool {id}: it no longer exists");
                        continue;
                    }
                    Err(e) => return Err(e.into()),
                },
            });
        }

        for row in rows {
            let sig = ToolSignature::from(row.clone());
            match CompiledUserTool::build(
                ToolSignature::from(row),
                Arc::clone(&weaver_registry),
                Arc::clone(&state),
                policy.actor.clone(),
                policy.realm.clone(),
            )
            .map(|tool| tool.with_modules(Arc::clone(&module_values)))
            {
                Ok(tool) => {
                    tracing::info!("Registering tool: {}", sig.name);
                    registry.register(tool)
                }
                Err(e) => tracing::warn!(%owner, "skipping uncompilable user tool: {e}"),
            }
        }

        let mut tool_names: Vec<String> = registry.defs().into_iter().map(|d| d.name).collect();
        tool_names.sort();

        let http_client = Arc::new(reqwest::Client::new());
        let stream = match connection.profile.provider.as_str() {
            "openrouter" => {
                let Some(api_key) = api_key else {
                    return Err(ChatError::Connection("no api key".into()));
                };
                let pp: OpenRouterOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                let provider = OpenRouterProvider::new(http_client, api_key.as_str(), model)
                    .with_request_modifier(|b| {
                        b.header("X-OpenRouter-Title", "Lorealm")
                            .header("HTTP-Referer", "https://github.com/Lorealm/Lorealm")
                    });
                run_inference(
                    provider,
                    messages,
                    common,
                    pp,
                    registry,
                    widget_sink,
                    effect_sink,
                    Arc::clone(&self.plugins),
                    owner,
                )
                .await
            }
            // Any OpenAI-compatible endpoint, including local servers such as
            // KoboldCPP, which need no key at all.
            "openai" => {
                let pp: OpenAiOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                let provider = OpenAiProvider::new(
                    http_client,
                    model,
                    pp.base_url.clone(),
                    api_key.map(|key| key.to_string()),
                );
                run_inference(
                    provider,
                    messages,
                    common,
                    pp,
                    registry,
                    widget_sink,
                    effect_sink,
                    Arc::clone(&self.plugins),
                    owner,
                )
                .await
            }
            "anthropic" => {
                let Some(api_key) = api_key else {
                    return Err(ChatError::Connection("no api key".into()));
                };
                let pp: AnthropicOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                let provider = AnthropicProvider::new(
                    http_client,
                    model,
                    pp.base_url.clone(),
                    api_key.as_str(),
                );
                run_inference(
                    provider,
                    messages,
                    common,
                    pp,
                    registry,
                    widget_sink,
                    effect_sink,
                    Arc::clone(&self.plugins),
                    owner,
                )
                .await
            }
            "vertex" => {
                let Some(api_key) = api_key else {
                    return Err(ChatError::Connection("no api key".into()));
                };
                let pp: VertexOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                // A profile holds one long-lived secret, which rules out
                // access tokens: they expire within the hour.
                let provider = VertexProvider::new(
                    http_client,
                    model,
                    VertexAuth::ApiKey(api_key.to_string()),
                    pp.project_id.clone(),
                    pp.location.clone(),
                );
                run_inference(
                    provider,
                    messages,
                    common,
                    pp,
                    registry,
                    widget_sink,
                    effect_sink,
                    Arc::clone(&self.plugins),
                    owner,
                )
                .await
            }
            other => Err(ChatError::Connection(format!(
                "unsupported provider: {other}"
            ))),
        }?;
        Ok((tool_names, stream))
    }

    /// Run one completion to its end and hand back the text.
    ///
    /// Persists nothing, streams nothing to a client, and exposes no tools.
    pub async fn complete(
        &self,
        owner: Snowflake,
        profile_id: Snowflake,
        prompt: Vec<PromptMessage>,
        cancel: CancellationToken,
    ) -> Result<String, ChatError> {
        // Assembly writes state; a bare completion has no assembly, so the
        // pool exists only to satisfy `infer` and is discarded.
        let state = Arc::new(StdMutex::new(StatePool::new()));
        let policy = ToolPolicy {
            tool_ids: Some(Vec::new()),
            ..Default::default()
        };

        let (_, stream) = self
            .infer(owner, profile_id, prompt, state, &policy)
            .await?;
        futures_util::pin_mut!(stream);

        let mut out = String::new();
        loop {
            let next = tokio::select! {
                biased;
                _ = cancel.cancelled() => None,
                next = stream.next() => next,
            };
            let Some(event) = next else { break };
            match event? {
                InferEvent::Text(delta) => out.push_str(&delta),
                // Reasoning is the model thinking, not the model answering;
                // widgets, tool calls and effects cannot arise with no tools
                // offered.
                InferEvent::Reasoning(_)
                | InferEvent::Widget(_)
                | InferEvent::ToolCall(_)
                | InferEvent::Effect(_) => {}
            }
        }
        Ok(out)
    }

    pub async fn complete_with_context(
        &self,
        owner: Snowflake,
        conversation_id: Snowflake,
        ask: String,
        profile_id: Snowflake,
        cancel: CancellationToken,
    ) -> Result<String, ChatError> {
        let convo = self.get_conversation(conversation_id, owner).await?;
        let cast = self.cast_of(conversation_id).await?;
        let agent = cast
            .agents
            .iter()
            .find(|a| a.kind == AgentKind::Llm)
            .ok_or(ChatError::NotFound(TargetType::Character, conversation_id))?;

        let (spec, _) = self.spec_for(owner, &cast, agent).await?;
        let path = self.storage.get_active_path(conversation_id).await?;
        let turns = perceive(&cast, agent, &path, &mut WidgetRenderer::default());
        let ids: Vec<Snowflake> = path.iter().map(|m| m.id).collect();
        let baseline = self.state.reconstruct(&ids).await?;
        let view = Stage::new(&cast, &path, &baseline.persistent).view;

        let Prepared {
            prompt: mut messages,
            ..
        } = self
            .assemble_context(
                &spec,
                &cast,
                agent,
                &view,
                Some(conversation_id),
                &turns,
                &baseline,
                &[],
                &[],
                &convo.author_note,
            )
            .await?;

        messages.push(PromptMessage {
            role: PromptRole::User,
            content: ask,
        });

        self.complete(owner, profile_id, messages, cancel).await
    }
}

fn prompt_role_to_provider_role(role: PromptRole) -> lorealm_providers::Role {
    match role {
        PromptRole::System => lorealm_providers::Role::System,
        PromptRole::User => lorealm_providers::Role::User,
        PromptRole::Assistant => lorealm_providers::Role::Assistant,
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_inference<P>(
    provider: P,
    mut messages: Vec<lorealm_providers::Message>,
    common: lorealm_providers::CommonParams,
    provider_params: P::ProviderParams,
    registry: lorealm_providers::ToolRegistry,
    widget_sink: WidgetSink,
    effect_sink: EffectSink,
    plugins: Arc<PluginService>,
    owner: Snowflake,
) -> Result<Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>, ChatError>
where
    P: LlmProvider + 'static,
    P::ProviderParams: Clone + Send + Sync + 'static,
{
    let client = Client::new(provider);
    let tool_defs = registry.defs();

    // TODO: merge into connection profiles?
    const MAX_TOOL_ROUNDS: usize = 8;

    let stream = async_stream::try_stream! {
        for round in 0..MAX_TOOL_ROUNDS {
            let mut upstream = client
                .request()
                .messages(messages.clone())
                .common_params(common.clone())
                .tools(tool_defs.clone())
                .configure(|p| *p = provider_params.clone())
                .stream()
                .await
                .map_err(ChatError::from)?;

            let mut acc = BlockAccumulator::default();

            while let Some(delta) = upstream.next().await {
                let delta = delta.map_err(ChatError::from)?;
                if !delta.reasoning.is_empty() {
                    yield InferEvent::Reasoning(delta.reasoning.clone());
                }
                if !delta.content.is_empty() {
                    yield InferEvent::Text(delta.content.clone());
                }
                acc.push(delta);
            }

            let (blocks, _) = acc.finish().map_err(ChatError::from)?;
            let uses: Vec<(String, String, serde_json::Value)> = blocks
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse { id, name, input } => {
                        Some((id.clone(), name.clone(), input.clone()))
                    }
                    _ => None,
                })
                .collect();

            if uses.is_empty() {
                break;
            }

            messages.push(lorealm_providers::Message {
                role: lorealm_providers::Role::Assistant,
                content: blocks,
            });

            // Execute and feed results back; surface widgets immediately.
            let mut results = Vec::new();
            for (id, name, input) in uses {
                let t0 = tokio::time::Instant::now();
                let (input, result) = match plugins.tool_call(owner, &name, input.clone()).await {
                    Ok(input) => {
                        let result = registry.call(&name, input.clone()).await;
                        (input, result.map_err(|e| e.to_string()))
                    }
                    Err(refusal) => (input, Err(refusal.to_string())),
                };
                let result = plugins.tool_result(owner, &name, &input, result).await;
                let (block, output, is_error) = match result {
                        Ok(value) => (
                            ContentBlock::ToolResult { tool_use_id: id.clone(), content: value.to_string(), is_error: false },
                            value, false,
                        ),
                        Err(e) => (
                            ContentBlock::ToolResult { tool_use_id: id.clone(), content: e.clone(), is_error: true },
                            serde_json::Value::String(e), true,
                        ),
                    };
                results.push(block);

                yield InferEvent::ToolCall(ToolCallRecord {
                        round, id, name, input, output, is_error,
                        duration_ms: t0.elapsed().as_millis() as u64,
                    });

                let attached: Vec<WidgetInstance> =
                    widget_sink.lock().unwrap().drain(..).collect();
                for widget in attached {
                    yield InferEvent::Widget(widget);
                }

                // Drained per call, like widgets, so a tool's effects reach the
                // client while the turn is still running.
                let effects: Vec<CommandEffect> =
                    effect_sink.lock().unwrap().drain(..).collect();
                for effect in effects {
                    yield InferEvent::Effect(effect);
                }
            }
            messages.push(lorealm_providers::Message {
                role: lorealm_providers::Role::User,
                content: results,
            });
        }
    };

    Ok(Box::pin(stream))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::Harness;
    use lorealm_providers::{Message, Role, ToolDef, ToolRegistry};

    struct Add;

    #[async_trait::async_trait]
    impl lorealm_providers::Tool for Add {
        fn def(&self) -> ToolDef {
            ToolDef {
                name: "add".into(),
                description: "Add".into(),
                input_schema: json!({"type": "object"}),
            }
        }
        async fn call(&self, _: serde_json::Value) -> lorealm_providers::Result<serde_json::Value> {
            Ok(json!(3))
        }
    }

    /// A tool round must hand back the model's signed reasoning: Anthropic and
    /// Gemini check it when the conversation continues.
    #[tokio::test]
    async fn the_tool_loop_replays_signed_reasoning() {
        let tool_round = concat!(
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\",\"signature\":\"\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Sum it.\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"sig\"}}\n\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"Adding.\"}}\n\n",
            "data: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"add\",\"input\":{}}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"}}\n\n",
        );
        let final_round = concat!(
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"3\"}}\n\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"}}\n\n",
        );

        let bodies: Arc<StdMutex<Vec<serde_json::Value>>> = Default::default();
        let seen = Arc::clone(&bodies);
        let endpoint = axum::Router::new().route(
            "/v1/messages",
            axum::routing::post(move |body: axum::Json<serde_json::Value>| async move {
                let mut seen = seen.lock().unwrap();
                seen.push(body.0);
                let sse = if seen.len() == 1 {
                    tool_round
                } else {
                    final_round
                };
                (
                    [(axum::http::header::CONTENT_TYPE, "text/event-stream")],
                    sse,
                )
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let base_url = format!("http://{}/v1", listener.local_addr().unwrap());
        tokio::spawn(async move { axum::serve(listener, endpoint).await.unwrap() });

        let h = Harness::new().await;
        let provider = AnthropicProvider::new(
            Arc::new(reqwest::Client::new()),
            "claude-opus-5",
            Some(base_url),
            "key",
        );
        let mut registry = ToolRegistry::new();
        registry.register(Add);

        let mut stream = run_inference(
            provider,
            vec![Message::text(Role::User, "1 + 2?")],
            Default::default(),
            AnthropicOptions::default(),
            registry,
            Default::default(),
            Default::default(),
            Arc::clone(&h.plugins),
            h.user,
        )
        .await
        .unwrap();

        let mut text = String::new();
        let mut reasoning = String::new();
        while let Some(event) = stream.next().await {
            match event.unwrap() {
                InferEvent::Text(t) => text.push_str(&t),
                InferEvent::Reasoning(r) => reasoning.push_str(&r),
                _ => {}
            }
        }
        assert_eq!(text, "Adding.3");
        assert_eq!(reasoning, "Sum it.");

        let bodies = bodies.lock().unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(
            bodies[1]["messages"][1],
            json!({"role": "assistant", "content": [
                {"type": "thinking", "thinking": "Sum it.", "signature": "sig"},
                {"type": "text", "text": "Adding."},
                {"type": "tool_use", "id": "toolu_1", "name": "add", "input": {}}
            ]})
        );
        assert_eq!(
            bodies[1]["messages"][2],
            json!({"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "toolu_1", "content": "3"}
            ]})
        );
    }
}
