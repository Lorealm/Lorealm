use crate::{
    models::*, storage::{CharacterStorage, ConversationStorage, MessageStorage, StorageCapabilities, StorageCore, UserStorage}, StorageError
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A wrapper around any storage backend that can be configured to fail on demand.
/// Useful for testing error handling and fallback scenarios.
#[derive(Debug)]
pub struct FailableStorage<S> {
    inner: S,
    should_fail: Arc<RwLock<bool>>,
    fail_on_init: Arc<RwLock<bool>>,
    fail_on_availability: Arc<RwLock<bool>>,
}

impl<S> FailableStorage<S> {
    pub fn new(storage: S) -> Self {
        Self {
            inner: storage,
            should_fail: Arc::new(RwLock::new(false)),
            fail_on_init: Arc::new(RwLock::new(false)),
            fail_on_availability: Arc::new(RwLock::new(false)),
        }
    }

    /// Make all operations fail
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.write().await = fail;
    }

    /// Make initialization fail
    pub async fn set_fail_on_init(&self, fail: bool) {
        *self.fail_on_init.write().await = fail;
    }

    /// Make availability check fail
    pub async fn set_fail_on_availability(&self, fail: bool) {
        *self.fail_on_availability.write().await = fail;
    }

    /// Reset all failure flags
    pub async fn reset(&self) {
        *self.should_fail.write().await = false;
        *self.fail_on_init.write().await = false;
        *self.fail_on_availability.write().await = false;
    }

    async fn check_failure(&self) -> Result<(), StorageError> {
        if *self.should_fail.read().await {
            Err(StorageError::Database("Simulated failure".to_string()))
        } else {
            Ok(())
        }
    }

    /// Get access to the inner storage (useful for setup/teardown)
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

#[async_trait]
impl<S: StorageCore> StorageCore for FailableStorage<S> {
    async fn is_available(&self) -> bool {
        if *self.fail_on_availability.read().await {
            return false;
        }
        self.inner.is_available().await
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        if *self.fail_on_init.read().await {
            return Err(StorageError::Database("Simulated init failure".to_string()));
        }
        self.inner.initialize().await
    }

    fn backend_type(&self) -> &str {
        self.inner.backend_type()
    }
}

#[async_trait]
impl<S: ConversationStorage> ConversationStorage for FailableStorage<S> {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        self.check_failure().await?;
        self.inner.save_conversation(conversation).await
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        self.check_failure().await?;
        self.inner.get_conversation(id).await
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        self.check_failure().await?;
        self.inner.list_conversations(user_id).await
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.delete_conversation(id).await
    }
}

#[async_trait]
impl<S: MessageStorage> MessageStorage for FailableStorage<S> {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        self.check_failure().await?;
        self.inner.save_message(message).await
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        self.check_failure().await?;
        self.inner.get_messages(conversation_id).await
    }
}

#[async_trait]
impl<S: UserStorage> UserStorage for FailableStorage<S> {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        self.check_failure().await?;
        self.inner.save_user(user).await
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        self.check_failure().await?;
        self.inner.get_user_by_id(id).await
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        self.check_failure().await?;
        self.inner.get_user_by_name(name).await
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        self.check_failure().await?;
        self.inner.save_session(session).await
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        self.check_failure().await?;
        self.inner.get_session_by_id(id).await
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        self.check_failure().await?;
        self.inner.get_session_by_token(token).await
    }

    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner
            .update_session_activity(session_id, last_active_at, new_expiration)
            .await
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.revoke_session(session_id).await
    }
    
    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.update_csrf_token(session_id, new_csrf_token, rotation_count, rotated_at).await
    }
}

#[async_trait]
impl<S: CharacterStorage> CharacterStorage for FailableStorage<S> {
    async fn get_character(&self, id: Uuid) -> Result<Option<Character>, StorageError> {
        self.check_failure().await?;
        self.inner.get_character(id).await
    }

    async fn get_character_tags(&self, id: Uuid) -> Result<Vec<CharacterTag>, StorageError> {
        self.check_failure().await?;
        self.inner.get_character_tags(id).await
    }

    async fn get_character_greetings(&self, id: Uuid) -> Result<Vec<CharacterGreeting>, StorageError> {
        self.check_failure().await?;
        self.inner.get_character_greetings(id).await
    }

    async fn get_characters_with_tags(&self, tags: Vec<String>) -> Result<Vec<Character>, StorageError> {
        self.check_failure().await?;
        self.inner.get_characters_with_tags(tags).await
    }

    async fn save_character(&self, character: &Character) -> Result<Uuid, StorageError> {
        self.check_failure().await?;
        self.inner.save_character(character).await
    }

    async fn save_character_tag(&self, tag: &CharacterTag) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.save_character_tag(tag).await
    }

    async fn save_character_greeting(&self, greeting: &CharacterGreeting) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.save_character_greeting(greeting).await
    }

    async fn delete_character(&self, id: Uuid) -> Result<(), StorageError> {
        self.check_failure().await?;
        self.inner.delete_character(id).await
    }

    async fn list_characters(&self, user_id: Uuid) -> Result<Vec<Character>, StorageError> {
        self.check_failure().await?;
        self.inner.list_characters(user_id).await
    }
}

impl<S: StorageCapabilities> StorageCapabilities for FailableStorage<S> {
    fn supports_conversations(&self) -> bool {
        self.inner.supports_conversations()
    }

    fn supports_messages(&self) -> bool {
        self.inner.supports_messages()
    }

    fn supports_users(&self) -> bool {
        self.inner.supports_users()
    }

    fn supports_sessions(&self) -> bool {
        self.inner.supports_sessions()
    }
}

// Clone implementation if inner storage is Clone
impl<S: Clone> Clone for FailableStorage<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            should_fail: Arc::clone(&self.should_fail),
            fail_on_init: Arc::clone(&self.fail_on_init),
            fail_on_availability: Arc::clone(&self.fail_on_availability),
        }
    }
}
