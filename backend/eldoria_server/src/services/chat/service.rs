use std::{pin::Pin, sync::Arc};

use chrono::Utc;
use eldoria_providers::{Client, OpenRouterProvider};
use eldoria_storage::{
    Storage,
    models::{ChatRole, Conversation, ConversationMessage, ConversationMetadata, Snowflake},
};
use futures_util::{Stream, StreamExt};
use serde_json::json;
use tokio::sync::Mutex;

use crate::{
    errors::{ChatError, TargetType},
    services::{
        character::CharacterService,
        chat::{ChatStreamEvent, FullConversation},
        context::{
            PromptRole, ResolveContext, ResolverRegistry, TracedBlock, assemble,
            builtin::register_builtin_resolvers,
        },
        conversation::{ConversationService, EditMode},
        snowflake::SnowflakeService,
        state::{CapturedState, StateService},
    },
};

enum Persist {
    Child(Snowflake),
    Sibling(Snowflake),
}

pub struct ChatService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    conversations: Arc<ConversationService>,
    characters: Arc<CharacterService>,
    state: Arc<StateService>,
    resolvers: Arc<ResolverRegistry>,
}

impl ChatService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        conversations: Arc<ConversationService>,
        characters: Arc<CharacterService>,
        state: Arc<StateService>,
    ) -> Self {
        let mut registry = ResolverRegistry::new();
        register_builtin_resolvers(&mut registry, Arc::clone(&state));

        Self {
            storage,
            snowflake,
            conversations,
            characters,
            state,
            resolvers: Arc::new(registry),
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
        user_id: Snowflake
    ) -> Result<ConversationMessage, ChatError> {
        let _convo = self.get_conversation(conversation_id, user_id).await?;
        let message = self.storage.get_message(message_id).await?.ok_or(ChatError::NotFound(TargetType::Message, message_id))?;
        if message.conversation_id != conversation_id {
            return Err(ChatError::NotFound(TargetType::Message, message_id));
        }
        Ok(message)
    }

    pub async fn get_path_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
        user_id: Snowflake
    ) -> Result<Vec<ConversationMessage>, ChatError> {
        let _convo = self.get_conversation(conversation_id, user_id).await?;
        let messages = self.storage.get_path_to(conversation_id, message_id).await?;

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

    pub async fn generate(
        &self,
        convo_id: Snowflake,
        user_id: Snowflake,
        content: Option<String>,
        target: Option<Snowflake>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<ChatStreamEvent, ChatError>> + Send>>, ChatError>
    {
        let convo = self.get_conversation(convo_id, user_id).await?;

        let assembly_leaf: Option<Snowflake>;
        let created_user: Option<ConversationMessage>;
        let persist: Persist;

        match (content, target) {
            // Normal turn.
            (Some(text), None) => {
                let user = self
                    .conversations
                    .append_message(convo_id, ChatRole::User, text, json!({}))
                    .await?;
                assembly_leaf = Some(user.id);
                persist = Persist::Child(user.id);
                created_user = Some(user);
            }
            // User-message swipe commit: user sibling of `target`, then reply.
            (Some(text), Some(t)) => {
                self.owned_message(t, convo_id).await?;
                let user = self
                    .conversations
                    .create_alternate(convo_id, t, ChatRole::User, text, json!({}))
                    .await?;
                assembly_leaf = Some(user.id);
                persist = Persist::Child(user.id);
                created_user = Some(user);
            }
            // Assistant-message swipe.
            (None, Some(t)) => {
                let msg = self.owned_message(t, convo_id).await?;
                assembly_leaf = msg.parent;
                persist = Persist::Sibling(t);
                created_user = None;
            }
            // Continue / regenerate from the leaf.
            (None, None) => {
                let leaf = convo
                    .active_leaf_id
                    // TODO: a ChatError::EmptyConversation variant reads better.
                    .ok_or(ChatError::NotFound(TargetType::Message, convo_id))?;
                assembly_leaf = Some(leaf);
                persist = Persist::Child(leaf);
                created_user = None;
            }
        }

        let thread = self.build_prompt(convo_id, assembly_leaf).await?;

        let character = self
            .characters
            .load_version(convo.character_id, convo.char_version_id, user_id)
            .await?;
        let template = self.state.get_active_template(user_id).await?;

        let captured: Option<CapturedState>;
        let state_service = self.state.clone();
        let trace: Vec<TracedBlock> = {
            let ctx = ResolveContext {
                character: &character,
                messages: &thread,
                user_id,
                lorebook_state: Mutex::new(None),
            };
            let trace = assemble(&template, &self.resolvers, &ctx).await?;

            captured = ctx.lorebook_state.lock().await.take();
            trace
        };

        let mut ephemeral_prompt: Vec<super::PromptMessage> = Vec::with_capacity(trace.len());

        for traced in &trace {
            ephemeral_prompt.push(super::PromptMessage {
                role: traced.block.role.clone(),
                content: traced.block.content.clone(),
            });
        }

        let token_stream = self.infer(ephemeral_prompt).await?;

        // Forward deltas, accumulate, persist on completion.
        let conversations = self.conversations.clone();
        let stream = async_stream::try_stream! {
            let mut acc = String::new();
            futures_util::pin_mut!(token_stream);

            if let Some(user) = created_user {
                yield ChatStreamEvent::UserMessage { message: user };
            }

            while let Some(delta) = token_stream.next().await {
                let delta = delta?;
                acc.push_str(&delta);
                yield ChatStreamEvent::Delta { content: delta };
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
            let meta = json!({
                "context": stored,
            });

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

    /// Token stream from the model.
    async fn infer(
        &self,
        prompt: Vec<super::PromptMessage>,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String, ChatError>> + Send>>, ChatError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| ChatError::ConnectionProfile("missing api key".to_string()))?;
        let model = std::env::var("OPENROUTER_MODEL")
            .map_err(|_| ChatError::ConnectionProfile("missing model".to_string()))?;

        let http_client = Arc::new(reqwest::Client::new());
        let provider = OpenRouterProvider::new(http_client, api_key, model);
        let client = Client::new(provider);

        // Map your custom payload directly to what the provider client expects
        let messages: Vec<eldoria_providers::Message> = prompt
            .into_iter()
            .map(|m| eldoria_providers::Message {
                role: prompt_role_to_provider_role(m.role),
                content: m.content,
            })
            .collect();

        let upstream = client.request().messages(messages).stream().await?;
        let mapped = upstream.map(|delta| delta.map(|d| d.content).map_err(ChatError::from));

        Ok(Box::pin(mapped))
    }
}

fn prompt_role_to_provider_role(role: PromptRole) -> eldoria_providers::Role {
    match role {
        PromptRole::System => eldoria_providers::Role::System,
        PromptRole::User => eldoria_providers::Role::User,
        PromptRole::Assistant => eldoria_providers::Role::Assistant,
    }
}
