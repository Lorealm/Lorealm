use std::{pin::Pin, sync::Arc};

use chrono::Utc;
use futures_util::{Stream, StreamExt};
use lorealm_providers::{Client, ContentBlock, LlmProvider, OpenRouterOptions, OpenRouterProvider};
use lorealm_storage::{
    Storage,
    models::{ChatRole, Conversation, ConversationMessage, ConversationMetadata, Snowflake},
};
use serde_json::json;
use tokio::sync::Mutex;

use crate::{
    errors::{ChatError, TargetType},
    services::{
        character::CharacterService,
        chat::{
            ChatStreamEvent, FullConversation, Persist, Prepared, PromptMessage, TurnPlan,
            widgets::{InferEvent, WidgetInstance, WidgetSink, widget_marker, widget_registry},
        },
        connection::ConnectionService,
        context::{
            PromptRole, ResolveContext, ResolverRegistry, TracedBlock, assemble,
            builtin::register_builtin_resolvers,
        },
        conversation::{ConversationService, EditMode},
        persona::PersonaService,
        schedule::{Schedule, StageCx, stages::PrivacyStage},
        snowflake::SnowflakeService,
        state::StateService,
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
    resolvers: Arc<ResolverRegistry>,
    schedule: Arc<Schedule>,
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
            resolvers: Arc::new(registry),
            schedule: Arc::new(Schedule::new(vec![Box::new(PrivacyStage)])),
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
            character_id,
            char_version_id: v_id,
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

    async fn prepare_prompt(
        &self,
        convo: &Conversation,
        user_id: Snowflake,
        assembly_leaf: Option<Snowflake>,
    ) -> Result<Prepared, ChatError> {
        let thread = self.build_prompt(convo.id, assembly_leaf).await?;
        let character = self
            .characters
            .load_version(convo.character_id, convo.char_version_id, user_id)
            .await?;
        let persona = self.state.get_active_persona(user_id).await?;
        let template = self.state.get_active_template(user_id).await?;

        let captured;
        let trace = {
            let ctx = ResolveContext {
                character: &character,
                messages: &thread,
                user_id,
                lorebook_state: Mutex::new(None),
                persona: &persona,
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
        convo: &Conversation,
    ) -> Result<StageCx, ChatError> {
        let active_persona = self.state.get_active_persona(user_id).await?;
        let actual_persona = self
            .personas
            .get_persona_for_character(user_id, convo.character_id, active_persona.id)
            .await?;

        Ok(StageCx {
            persona: actual_persona,
        })
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

        let TurnPlan {
            assembly_leaf,
            created_user,
            persist,
        } = self.resolve_turn(&convo, content, target).await?;

        let Prepared {
            mut prompt,
            trace,
            captured,
        } = self.prepare_prompt(&convo, user_id, assembly_leaf).await?;

        // Outbound transform pass; hands back the inbound chain (reverse order).
        let cx = self.stage_cx(user_id, &convo).await?;
        let mut chain = self.schedule.outbound(&mut prompt, &cx);

        let token_stream = self.infer(user_id, prompt).await?;

        let conversations = self.conversations.clone();
        let state_service = self.state.clone();
        let stream = async_stream::try_stream! {
            let mut acc = String::new();
            let mut reasoning_acc = String::new();
            let mut widgets: Vec<WidgetInstance> = Vec::new();
            futures_util::pin_mut!(token_stream);

            if let Some(user) = created_user {
                yield ChatStreamEvent::UserMessage { message: user };
            }

            while let Some(event) = token_stream.next().await {
                match event? {
                    InferEvent::Text(delta) => {
                        let emitted = chain.push(&delta);
                        if !emitted.is_empty() {
                            acc.push_str(&emitted);
                            yield ChatStreamEvent::Delta { content: emitted };
                        }
                    }
                    InferEvent::Reasoning(delta) => {
                        let filtered = chain.apply(&delta);
                        if !filtered.is_empty() {
                            reasoning_acc.push_str(&filtered);
                            yield ChatStreamEvent::Reasoning { content: filtered };
                        }
                    }
                    InferEvent::Widget(widget) => {
                        let widget = widget.filtered(|s| chain.apply(s));
                        yield ChatStreamEvent::Widget { widget: widget.clone() };
                        // Record the widget's position in the text: the marker
                        // goes into the persisted content AND out as a normal delta
                        let marker = widget_marker(widgets.len());
                        acc.push_str(&marker);
                        yield ChatStreamEvent::Delta { content: marker };
                        widgets.push(widget);
                    }
                }
            }
            let tail = chain.finish();
            if !tail.is_empty() {
                acc.push_str(&tail);
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
            let meta = json!({ "context": stored, "widgets": widgets, "reasoning": reasoning_acc });

            let message = match persist {
                Persist::Child(parent) =>
                    conversations.branch_from(convo_id, parent, ChatRole::Assistant, acc, meta).await?,
                Persist::Sibling(sibling) =>
                    conversations.create_alternate(convo_id, sibling, ChatRole::Assistant, acc, meta).await?,
            };

            if let Some(captured) = captured {
                state_service.persist(message.id, captured).await?;
            }

            yield ChatStreamEvent::Done { message };
        };

        Ok(Box::pin(stream))
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
        user_id: Snowflake,
        prompt: Vec<super::PromptMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>, ChatError> {
        let profile = self.state.get_active_profile(user_id).await?;
        let connection = self
            .connection
            .resolve_connection(user_id, profile.id)
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

        let (registry, widget_sink) = widget_registry();

        let http_client = Arc::new(reqwest::Client::new());
        match connection.profile.provider.as_str() {
            "openrouter" => {
                let pp: OpenRouterOptions =
                    serde_json::from_value(connection.profile.provider_params.clone())
                        .map_err(|e| ChatError::Connection(format!("bad provider params: {e}")))?;
                let provider = OpenRouterProvider::new(http_client, api_key.as_str(), model)
                    .with_request_modifier(|b| b.header("X-OpenRouter-Title", "Lorealm"));
                run_inference(provider, messages, common, pp, registry, widget_sink).await
            }
            other => Err(ChatError::Connection(format!(
                "unsupported provider: {other}"
            ))),
        }
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
        for _ in 0..MAX_TOOL_ROUNDS {
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
                let block = match registry.call(&name, input).await {
                    Ok(value) => ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: value.to_string(),
                        is_error: false,
                    },
                    Err(e) => ContentBlock::ToolResult {
                        tool_use_id: id,
                        content: e.to_string(),
                        is_error: true,
                    },
                };
                results.push(block);

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
