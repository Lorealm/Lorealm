use crate::StorageError;
use crate::models::*;
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use uuid::Uuid;

/// Core trait - must be implemented by all backends
#[async_trait]
pub trait StorageCore: Send + Sync + std::fmt::Debug {
    async fn is_available(&self) -> bool;
    async fn initialize(&mut self) -> Result<(), StorageError>;
    fn backend_type(&self) -> &str;
}

/// Conversation operations
#[allow(unused_variables)]
#[async_trait]
pub trait ConversationStorage: StorageCore {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support conversations",
            self.backend_type()
        )))
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support conversations",
            self.backend_type()
        )))
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support conversations",
            self.backend_type()
        )))
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support conversations",
            self.backend_type()
        )))
    }
}

/// Message operations
#[allow(unused_variables)]
#[async_trait]
pub trait MessageStorage: StorageCore {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support messages",
            self.backend_type()
        )))
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support messages",
            self.backend_type()
        )))
    }
}

/// User and session operations
#[allow(unused_variables)]
#[async_trait]
pub trait UserStorage: StorageCore {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support users",
            self.backend_type()
        )))
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support users",
            self.backend_type()
        )))
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support users",
            self.backend_type()
        )))
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }

    async fn invalidate_sessions(
        &self,
        user_id: Uuid,
        ip: &str,
        user_agent: &str,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support sessions",
            self.backend_type()
        )))
    }
}

#[allow(unused_variables)]
#[async_trait]
pub trait CharacterStorage: StorageCore {
    async fn get_character(&self, id: Uuid) -> Result<Option<Character>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn get_character_tags(&self, id: Uuid) -> Result<Vec<CharacterTag>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn get_character_greetings(
        &self,
        id: Uuid,
    ) -> Result<Vec<CharacterGreeting>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn get_characters_with_tags(
        &self,
        tags: Vec<String>,
    ) -> Result<Vec<Character>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn save_character(&self, character: &Character) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn save_character_tag(&self, tag: &CharacterTag) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn save_character_greeting(
        &self,
        greeting: &CharacterGreeting,
    ) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn delete_character(&self, id: Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }

    async fn list_characters(&self, user_id: Uuid) -> Result<Vec<Character>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support characters",
            self.backend_type()
        )))
    }
}

/// Template storage operations
#[allow(unused_variables)]
#[async_trait]
pub trait TemplateStorage: StorageCore {
    /// Save a template (insert or update)
    async fn save_template(&self, template: &Template) -> Result<Uuid, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// Get a template by ID
    async fn get_template(&self, id: Uuid) -> Result<Option<Template>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// Get a template by ID, but only if owned by the specified user
    async fn get_template_owned(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Template>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// List all templates for a user (metadata only)
    async fn list_templates(&self, owner_id: Uuid) -> Result<Vec<TemplateMetadata>, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// Delete a template
    async fn delete_template(&self, id: Uuid, owner_id: Uuid) -> Result<(), StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// Check if a template exists
    async fn template_exists(&self, id: Uuid) -> Result<bool, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }

    /// Check if a template name is already used by this owner
    async fn template_name_exists(
        &self,
        owner_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, StorageError> {
        Err(StorageError::NotImplemented(format!(
            "{} does not support templates",
            self.backend_type()
        )))
    }
}

/// Combined trait for backwards compatibility
pub trait Storage:
    StorageCore + ConversationStorage + MessageStorage + UserStorage + CharacterStorage + TemplateStorage
{
}

// Blanket implementation
impl<T> Storage for T where
    T: StorageCore + ConversationStorage + MessageStorage + UserStorage + CharacterStorage + TemplateStorage
{
}

pub trait StorageCapabilities {
    fn supports_conversations(&self) -> bool {
        false
    }
    fn supports_messages(&self) -> bool {
        false
    }
    fn supports_users(&self) -> bool {
        false
    }
    fn supports_sessions(&self) -> bool {
        false
    }
    fn supports_characters(&self) -> bool {
        false
    }
    fn supports_templates(&self) -> bool {
        false
    }
}

pub enum ItemSortType {
    Id,
    LatestUpdate,
    LatestAdded,
    Random(i32), // Randomized with reproducible seed
    Tags,        // To be implemented
}

/// Optional trait for backends that support advanced features
#[async_trait]
pub trait AdvancedStorage: Storage {
    /// Full-text search across conversations
    async fn search_conversations(
        &self,
        query: &str,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError>;

    async fn get_conversations_paginated(
        &self,
        sort_type: ItemSortType,
        user_id: Uuid,
        page_size: usize,
        page: usize,
    ) -> Result<Vec<ConversationMetadata>, StorageError>;
}
