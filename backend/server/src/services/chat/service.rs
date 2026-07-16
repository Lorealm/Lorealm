use std::{collections::HashMap, pin::Pin, sync::Arc};

use chrono::Utc;
use dashmap::DashMap;
use futures_util::{Stream, StreamExt};
use lorealm_providers::{Client, ContentBlock, LlmProvider, OpenRouterOptions, OpenRouterProvider};
use lorealm_storage::{
    Storage,
    models::{
        AgentSpec, ChatRole, Conversation, ConversationMessage, ConversationMetadata, Snowflake,
    },
};
use serde_json::json;
use std::sync::Mutex as StdMutex;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::{ChatError, TargetType},
    services::{
        character::CharacterService,
        chat::{
            CancelGuard, ChatStreamEvent, FullConversation, OpenAgentTurn, Persist, Prepared,
            PromptMessage, TurnAccumulator, TurnPlan,
            widgets::{InferEvent, ToolCallRecord, WidgetInstance, WidgetSink, widget_registry},
        },
        connection::ConnectionService,
        context::{
            ContextTemplate, PromptRole, PromptTurn, ResolveContext, ResolverRegistry, TracedBlock,
            assemble, builtin::register_builtin_resolvers,
        },
        conversation::{ConversationService, EditMode},
        persona::PersonaService,
        schedule::{Schedule, StageCx, stages::PrivacyStage},
        snowflake::SnowflakeService,
        state::{CapturedState, ReconstructedState, StatePool, StateService, apply_delta},
        tokenizer::TokenizerService,
        tools::{
            CompiledUserTool, ToolPolicy, ToolSignature, UserToolService, build_tool_registry,
        },
    },
};

pub struct ChatService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    conversations: Arc<ConversationService>,
    characters: Arc<CharacterService>,
    personas: Arc<PersonaService>,
    state: Arc<StateService>,
    connection: Arc<ConnectionService>,
    tools: Arc<UserToolService>,
    tokenizers: Arc<TokenizerService>,
    resolvers: Arc<ResolverRegistry>,
    schedule: Arc<Schedule>,
    cancellations: Arc<DashMap<Snowflake, CancellationToken>>,
}

impl ChatService {
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
    ) -> Self {
        let mut registry = ResolverRegistry::new();
        register_builtin_resolvers(&mut registry, Arc::clone(&state));

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
            resolvers: Arc::new(registry),
            schedule: Arc::new(Schedule::new(vec![Box::new(PrivacyStage)])),
            cancellations: Arc::new(DashMap::new()),
        }
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

    async fn get_conversation(
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

    // TODO: There exists a bug where the lorebook state is not produced for the first turn
    // it should reside here, but I haven't looked into it
    pub async fn start_new(
        &self,
        character_id: Snowflake,
        version_id: Option<Snowflake>,
        user_id: Snowflake,
        greeting_id: Snowflake,
        content: String,
    ) -> Result<FullConversation, ChatError> {
        let v_id = version_id.unwrap_or(self.characters.get_latest_version_id(character_id).await?);
        let char = self
            .characters
            .load_version(character_id, v_id, user_id)
            .await?;

        let mut convo = Conversation {
            id: self.snowflake.generate(),
            user_id,
            character_id: Some(character_id),
            char_version_id: Some(v_id),
            realm_id: None,
            active_leaf_id: None,
            title: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.storage.create_conversation(&convo).await?;

        let greeting = char
            .definition
            .greetings
            .iter()
            .find(|g| g.id == greeting_id)
            .ok_or(ChatError::NotFound(TargetType::Greeting, greeting_id))?;

        self.conversations
            .append_message(
                convo.id,
                ChatRole::Assistant,
                greeting.text.clone(),
                json!({}),
            )
            .await?;

        let latest_id = self
            .conversations
            .append_message(convo.id, ChatRole::User, content, json!({}))
            .await?
            .id;

        // Lazy update to avoid a seprate database sync
        convo.active_leaf_id = Some(latest_id);

        let messages = self.storage.list_messages(convo.id).await?;

        Ok(FullConversation {
            metadata: convo,
            messages,
        })
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

    /// Swipe/branch a *user* message: create a sibling with new content that
    /// becomes the active leaf. Does NOT trigger inference.
    pub async fn branch_user_message(
        &self,
        convo_id: Snowflake,
        msg_id: Snowflake,
        user_id: Snowflake,
        content: String,
    ) -> Result<ConversationMessage, ChatError> {
        self.branch(convo_id, msg_id, user_id, content, json!({}))
            .await
    }

    async fn branch(
        &self,
        convo_id: Snowflake,
        msg_id: Snowflake,
        user_id: Snowflake,
        content: String,
        metadata: serde_json::Value,
    ) -> Result<ConversationMessage, ChatError> {
        let convo = self.get_conversation(convo_id, user_id).await?;

        let msg = self
            .storage
            .get_message(msg_id)
            .await?
            .ok_or(ChatError::NotFound(TargetType::Message, msg_id))?;

        Ok(self
            .conversations
            .create_alternate(convo.id, msg_id, msg.role, content, metadata)
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

    /// Message exists AND belongs to this conversation.
    async fn owned_message(
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

    async fn resolve_turn(
        &self,
        convo: &Conversation,
        content: Option<String>,
        target: Option<Snowflake>,
    ) -> Result<TurnPlan, ChatError> {
        Ok(match (content, target) {
            (Some(text), None) => {
                let user = self
                    .conversations
                    .append_message(convo.id, ChatRole::User, text, json!({}))
                    .await?;
                TurnPlan {
                    assembly_leaf: Some(user.id),
                    persist: Persist::Child(user.id),
                    created_user: Some(user),
                }
            }
            (Some(text), Some(t)) => {
                self.owned_message(t, convo.id).await?;
                let user = self
                    .conversations
                    .create_alternate(convo.id, t, ChatRole::User, text, json!({}))
                    .await?;
                TurnPlan {
                    assembly_leaf: Some(user.id),
                    persist: Persist::Child(user.id),
                    created_user: Some(user),
                }
            }
            (None, Some(t)) => {
                let msg = self.owned_message(t, convo.id).await?;
                TurnPlan {
                    assembly_leaf: msg.parent,
                    persist: Persist::Sibling(t),
                    created_user: None,
                }
            }
            (None, None) => {
                let leaf = convo
                    .active_leaf_id
                    .ok_or(ChatError::NotFound(TargetType::Message, convo.id))?;
                TurnPlan {
                    assembly_leaf: Some(leaf),
                    persist: Persist::Child(leaf),
                    created_user: None,
                }
            }
        })
    }

    async fn assemble_context(
        &self,
        spec: &AgentSpec,
        thread: &[PromptTurn],
        baseline: &ReconstructedState,
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

        let captured;
        let trace = {
            let ctx = ResolveContext {
                character: character.as_ref(),
                messages: &thread,
                user_id: spec.owner,
                state: baseline,
                lorebook_state: Mutex::new(None),
                lorebook_ids: spec.lorebook_ids.as_deref(),
                persona: persona.as_ref(),
                tokenizer: &tokenizer,
            };
            let trace = assemble(&template, &self.resolvers, &ctx).await?;
            captured = ctx.lorebook_state.lock().await.take();
            trace
        };

        let prompt = trace
            .iter()
            .map(|t| PromptMessage {
                role: t.block.role.clone(),
                content: t.block.content.clone(),
            })
            .collect();

        Ok(Prepared {
            prompt,
            trace,
            captured,
        })
    }

    async fn stage_cx(
        &self,
        user_id: Snowflake,
        character_id: Snowflake,
    ) -> Result<StageCx, ChatError> {
        let active_persona = self.state.get_active_persona(user_id).await?;
        let actual_persona = self
            .personas
            .get_persona_for_character(user_id, character_id, active_persona.id)
            .await?;

        Ok(StageCx {
            persona: actual_persona,
        })
    }

    /// The character bound to a plain (non-realm) conversation.
    fn convo_character(convo: &Conversation) -> Result<(Snowflake, Snowflake), ChatError> {
        match (convo.character_id, convo.char_version_id) {
            (Some(c), Some(v)) => Ok((c, v)),
            _ => Err(ChatError::NotFound(TargetType::Character, convo.id)),
        }
    }

    async fn active_spec(
        &self,
        user_id: Snowflake,
        convo: &Conversation,
    ) -> Result<AgentSpec, ChatError> {
        tracing::info!("evaluating spec");
        Ok(AgentSpec {
            owner: user_id,
            character: Some(Self::convo_character(convo)?),
            template_id: self.state.get_active_template_id(user_id).await?.ok_or(
                ChatError::State(crate::errors::StateError::NoActiveTemplate),
            )?,
            persona_id: Some(self.state.get_active_persona(user_id).await?.id),
            tokenizer_id: self.state.get_active_tokenizer_id(user_id).await?.ok_or(
                ChatError::State(crate::errors::StateError::NoActiveTokenizer),
            )?,
            profile_id: self.state.get_active_profile(user_id).await?.id,
            lorebook_ids: None,
        })
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

    pub async fn generate(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        content: Option<String>,
        target: Option<Snowflake>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, ChatError>> + Send>>, ChatError>
    {
        let convo = self.get_conversation(convo_id, user_id).await?;
        if convo.realm_id.is_some() {
            // Realm playthroughs advance through the grant-checked turn
            // pipeline, never through free-form completion.
            return Err(ChatError::RealmConversation(convo_id));
        }
        let (cancel, cancel_guard) = self.begin_cancel(convo_id);

        let TurnPlan {
            assembly_leaf,
            created_user,
            persist,
        } = self.resolve_turn(&convo, content, target).await?;

        let thread = self.build_prompt(convo_id, assembly_leaf).await?;
        let turns: Vec<PromptTurn> = thread.iter().map(PromptTurn::from_message).collect();
        let ids: Vec<Snowflake> = thread.iter().map(|m| m.id).collect();
        let baseline = self.state.reconstruct(&ids).await?;

        // "Use this user's active selections." A Realm agent supplies its own spec.
        let spec = self.active_spec(user_id, &convo).await?;

        let Prepared {
            mut prompt,
            trace,
            captured,
        } = self.assemble_context(&spec, &turns, &baseline).await?;

        let ids: Vec<Snowflake> = thread.iter().map(|m| m.id).collect();
        let baseline = self.state.reconstruct(&ids).await?.persistent;
        let mut working = baseline.clone();
        let (activation, had_assembly) = match captured {
            Some(c) => {
                apply_delta(&mut working, &c.persistent);
                (c.activation, true)
            }
            None => (HashMap::new(), false),
        };
        let state_pool = Arc::new(StdMutex::new(working));

        let cx = self
            .stage_cx(user_id, Self::convo_character(&convo)?.0)
            .await?;
        let mut chain_tf = self.schedule.outbound(&mut prompt, &cx);
        let token_stream = self
            .infer(
                &spec,
                prompt,
                Arc::clone(&state_pool),
                &ToolPolicy::default(),
            )
            .await?;

        let conversations = self.conversations.clone();
        let state_service = self.state.clone();

        let tokenizer_id = state_service.get_active_tokenizer_id(user_id).await?;
        let template_id = state_service.get_active_template_id(user_id).await?;
        let profile_id = state_service.get_active_profile(user_id).await?.id;

        let stream = async_stream::try_stream! {
            let _cancel_guard = cancel_guard;   // give the stream ownership
            let mut acc = TurnAccumulator::default();
            futures_util::pin_mut!(token_stream);

            if let Some(user) = created_user {
                yield ChatStreamEvent::UserMessage { message: user };
            }

            loop {
                tokio::select! {
                    biased;
                    _ = cancel.cancelled() => break,
                    next = token_stream.next() => {
                        let Some(Ok(event)) = next else { break };
                        match event {
                            InferEvent::Text(delta) => {
                                let emitted = chain_tf.push(&delta);
                                if !emitted.is_empty() {
                                    acc.text(&emitted);
                                    yield ChatStreamEvent::Delta { content: emitted };
                                }
                            }
                            InferEvent::Reasoning(delta) => {
                                let filtered = chain_tf.apply(&delta);
                                if !filtered.is_empty() {
                                    acc.reasoning(&filtered);
                                    yield ChatStreamEvent::Reasoning { content: filtered };
                                }
                            }
                            InferEvent::Widget(widget) => {
                                let widget = widget.filtered(|s| chain_tf.apply(s));
                                yield ChatStreamEvent::Widget { widget: widget.clone() };
                                let marker = acc.widget(widget);
                                yield ChatStreamEvent::Delta { content: marker };
                            }
                            InferEvent::ToolCall(rec) => acc.tool_call(rec),
                        }
                    }
                }
            }
            let tail = chain_tf.finish();
            if !tail.is_empty() {
                acc.text(&tail);
                yield ChatStreamEvent::Delta { content: tail };
            }

            let stored: Vec<TracedBlock> = trace
                .into_iter()
                .map(|mut t| {
                    if t.block.content_ref.is_recoverable() {
                        t.block.content.clear();
                    }
                    t
                })
                .collect();

            let outcome = acc.finish();
            // meta/persist now read from `outcome`:
            let meta = json!({
                "context": stored,
                "reasoning": outcome.reasoning,
                "tool_calls": outcome.tool_calls,
                "widgets": outcome.widgets,
                "conn_profile": profile_id,
                "tok_id": tokenizer_id,
                "template_id": template_id,
            });

            let message = match persist {
                Persist::Child(parent)    => conversations.branch_from(convo_id, parent, ChatRole::Assistant, outcome.spoken, meta).await?,
                Persist::Sibling(sibling) => conversations.create_alternate(convo_id, sibling, ChatRole::Assistant, outcome.spoken, meta).await?,
            };

            let delta = {
                let pool = state_pool.lock().unwrap();
                state_service.diff(&baseline, &pool)
            };

            if had_assembly || !delta.writes.is_empty() || !delta.deletes.is_empty() {
                let captured = CapturedState { activation, persistent: delta };
                state_service.persist(message.id, captured).await?;
            }

            yield ChatStreamEvent::Done { message };
        };

        Ok(Box::pin(stream))
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

    async fn build_prompt(
        &self,
        convo_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        let Some(leaf) = leaf else {
            return Ok(Vec::new());
        };

        let by_id: std::collections::HashMap<Snowflake, ConversationMessage> = self
            .storage
            .list_messages(convo_id)
            .await?
            .into_iter()
            .map(|m| (m.id, m))
            .collect();

        let mut path = Vec::new();
        let mut cursor = Some(leaf);
        while let Some(id) = cursor {
            let Some(msg) = by_id.get(&id) else { break };
            path.push(msg.clone());
            cursor = msg.parent;
            if path.len() > by_id.len() {
                break;
            }
        }
        path.reverse();
        Ok(path)
    }

    /// Token + widget stream from the model. Runs the streaming tool loop:
    /// text deltas are yielded as they arrive; when a round ends in tool
    /// calls, they are executed, fed back, and another round is streamed.
    async fn infer(
        &self,
        spec: &AgentSpec,
        prompt: Vec<super::PromptMessage>,
        state: Arc<StdMutex<StatePool>>,
        policy: &ToolPolicy,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>, ChatError> {
        let profile = self
            .connection
            .get_profile(spec.owner, spec.profile_id)
            .await?;
        let connection = self
            .connection
            .resolve_connection(spec.owner, profile.id)
            .await?;

        let Some(api_key) = connection.api_key else {
            return Err(ChatError::Connection("no api key".into()));
        };
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

        let (mut registry, widget_sink) = widget_registry();

        let weaver_registry = Arc::new(build_tool_registry());

        let rows = match &policy.tool_ids {
            None => self.tools.list_enabled(spec.owner).await?,
            Some(ids) => {
                let mut rows = Vec::with_capacity(ids.len());
                for id in ids {
                    rows.push(self.tools.get_tool(*id, spec.owner).await?);
                }
                rows
            }
        };
        for row in rows {
            let sig = ToolSignature::from(row.clone());
            match CompiledUserTool::build(
                ToolSignature::from(row),
                Arc::clone(&weaver_registry),
                Arc::clone(&state),
                policy.write_scopes.clone(),
            ) {
                Ok(tool) => {
                    tracing::info!("Registering tool: {}", sig.name);
                    registry.register(tool)
                }
                Err(e) => tracing::warn!(%spec.owner, "skipping uncompilable user tool: {e}"),
            }
        }

        let http_client = Arc::new(reqwest::Client::new());
        match connection.profile.provider.as_str() {
            "openrouter" => {
                let pp: OpenRouterOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                let provider = OpenRouterProvider::new(http_client, api_key.as_str(), model)
                    .with_request_modifier(|b| {
                        b.header("X-OpenRouter-Title", "Lorealm")
                            .header("HTTP-Referer", "https://github.com/Lorealm/Lorealm")
                    });
                run_inference(provider, messages, common, pp, registry, widget_sink).await
            }
            other => Err(ChatError::Connection(format!(
                "unsupported provider: {other}"
            ))),
        }
    }

    /// Assemble context and start inference, handing the live event stream
    /// to the caller (used by realm tick streaming).
    pub async fn open_agent_turn(
        &self,
        spec: &AgentSpec,
        turns: &[PromptTurn],
        baseline: &ReconstructedState,
        policy: &ToolPolicy,
    ) -> Result<OpenAgentTurn, ChatError> {
        let Prepared {
            prompt,
            trace,
            captured,
        } = self.assemble_context(spec, turns, baseline).await?;

        let diff_base = baseline.persistent.clone();
        let mut working = baseline.persistent.clone();
        let (activation, had_assembly) = match &captured {
            Some(c) => {
                apply_delta(&mut working, &c.persistent);
                (c.activation.clone(), true)
            }
            None => (HashMap::new(), false),
        };
        let state_pool = Arc::new(StdMutex::new(working));

        let stream = self
            .infer(spec, prompt, Arc::clone(&state_pool), policy)
            .await?;

        Ok(OpenAgentTurn {
            stream,
            trace,
            state_pool,
            diff_base,
            activation,
            had_assembly,
        })
    }
}

fn prompt_role_to_provider_role(role: PromptRole) -> lorealm_providers::Role {
    match role {
        PromptRole::System => lorealm_providers::Role::System,
        PromptRole::User => lorealm_providers::Role::User,
        PromptRole::Assistant => lorealm_providers::Role::Assistant,
    }
}

async fn run_inference<P>(
    provider: P,
    mut messages: Vec<lorealm_providers::Message>,
    common: lorealm_providers::CommonParams,
    provider_params: P::ProviderParams,
    registry: lorealm_providers::ToolRegistry,
    widget_sink: WidgetSink,
) -> Result<Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>, ChatError>
where
    P: LlmProvider + 'static,
    P::ProviderParams: Clone + Send + Sync + 'static,
{
    let client = Client::new(provider);
    let tool_defs = registry.defs();

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

            let mut text = String::new();
            let mut calls: Vec<(Option<String>, Option<String>, String)> = Vec::new();

            while let Some(delta) = upstream.next().await {
                let delta = delta.map_err(ChatError::from)?;
                if !delta.reasoning.is_empty() {
                    yield InferEvent::Reasoning(delta.reasoning);
                }
                if !delta.content.is_empty() {
                    text.push_str(&delta.content);
                    yield InferEvent::Text(delta.content);
                }
                for tc in delta.tool_calls {
                    if calls.len() <= tc.index {
                        calls.resize(tc.index + 1, (None, None, String::new()));
                    }
                    let slot = &mut calls[tc.index];
                    if tc.id.is_some() {
                        slot.0 = tc.id;
                    }
                    if tc.name.is_some() {
                        slot.1 = tc.name;
                    }
                    slot.2.push_str(&tc.arguments);
                }
            }

            if calls.is_empty() {
                break;
            }

            // Record the assistant turn (text + tool uses) in the history.
            let mut blocks = Vec::new();
            if !text.is_empty() {
                blocks.push(ContentBlock::Text { text });
            }
            let mut uses = Vec::new();
            for (id, name, args) in calls {
                let input = if args.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::from_str(&args)
                        .map_err(|e| ChatError::from(lorealm_providers::Error::from(e)))?
                };
                let id = id.unwrap_or_default();
                let name = name.unwrap_or_default();
                blocks.push(ContentBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                });
                uses.push((id, name, input));
            }
            messages.push(lorealm_providers::Message {
                role: lorealm_providers::Role::Assistant,
                content: blocks,
            });

            // Execute and feed results back; surface widgets immediately.
            let mut results = Vec::new();
            for (id, name, input) in uses {
                let t0 = tokio::time::Instant::now();
                let (block, output, is_error) = match registry.call(&name, input.clone()).await {
                        Ok(value) => (
                            ContentBlock::ToolResult { tool_use_id: id.clone(), content: value.to_string(), is_error: false },
                            value, false,
                        ),
                        Err(e) => (
                            ContentBlock::ToolResult { tool_use_id: id.clone(), content: e.to_string(), is_error: true },
                            serde_json::Value::String(e.to_string()), true,
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
            }
            messages.push(lorealm_providers::Message {
                role: lorealm_providers::Role::User,
                content: results,
            });
        }
    };

    Ok(Box::pin(stream))
}
