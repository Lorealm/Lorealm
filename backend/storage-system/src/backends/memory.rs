use crate::error::StorageError;
use crate::{models::*, TemplateStorage};
use crate::storage::CharacterStorage;
use crate::{
    impl_storage_capabilities,
    storage::{ConversationStorage, MessageStorage, StorageCore, UserStorage},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// In-memory storage backend
///
/// This backend stores all data in memory using HashMaps. It's useful for:
/// - Testing
/// - Development
/// - Temporary storage
/// - Cache-like scenarios
///
/// Note: All data is lost when the process stops.
#[derive(Debug)]
pub struct MemoryStorage {
    conversations: Arc<RwLock<HashMap<Uuid, Conversation>>>,
    conversation_metadata: Arc<RwLock<HashMap<Uuid, ConversationMetadata>>>,
    messages: Arc<RwLock<HashMap<Uuid, Vec<Message>>>>,
    users: Arc<RwLock<HashMap<Uuid, User>>>,
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    templates: Arc<RwLock<HashMap<Uuid, Template>>>,
    max_size: usize,
    current_size: Arc<RwLock<usize>>,
}

impl MemoryStorage {
    /// Create a new memory storage with a maximum item limit
    ///
    /// # Arguments
    /// * `max_size` - Maximum number of items to store (0 = unlimited)
    pub fn new(max_size: usize) -> Self {
        Self {
            conversations: Arc::new(RwLock::new(HashMap::new())),
            conversation_metadata: Arc::new(RwLock::new(HashMap::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
            users: Arc::new(RwLock::new(HashMap::new())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            templates: Arc::new(RwLock::new(HashMap::new())),
            max_size,
            current_size: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if we can add more items
    async fn can_add_item(&self) -> bool {
        if self.max_size == 0 {
            return true; // Unlimited
        }

        let current = *self.current_size.read().await;
        current < self.max_size
    }

    /// Increment the item counter
    async fn increment_size(&self) {
        if self.max_size > 0 {
            let mut size = self.current_size.write().await;
            *size += 1;
        }
    }

    /// Decrement the item counter
    async fn decrement_size(&self) {
        if self.max_size > 0 {
            let mut size = self.current_size.write().await;
            if *size > 0 {
                *size -= 1;
            }
        }
    }

    /// Get current storage statistics
    pub async fn stats(&self) -> MemoryStorageStats {
        let conversations = self.conversations.read().await.len();
        let users = self.users.read().await.len();
        let messages_map = self.messages.read().await;
        let total_messages: usize = messages_map.values().map(|v| v.len()).sum();
        let templates = self.templates.read().await.len();

        MemoryStorageStats {
            conversation_count: conversations,
            user_count: users,
            message_count: total_messages,
            max_size: self.max_size,
            current_size: *self.current_size.read().await,
            template_count: templates
        }
    }

    /// Clear all data from storage
    pub async fn clear(&self) {
        self.conversations.write().await.clear();
        self.conversation_metadata.write().await.clear();
        self.messages.write().await.clear();
        self.users.write().await.clear();
        self.sessions.write().await.clear();
        self.templates.write().await.clear();
        *self.current_size.write().await = 0;
    }
}

/// Statistics about memory storage usage
#[derive(Debug, Clone)]
pub struct MemoryStorageStats {
    pub conversation_count: usize,
    pub user_count: usize,
    pub message_count: usize,
    pub max_size: usize,
    pub current_size: usize,
    pub template_count: usize,
}

#[async_trait]
impl StorageCore for MemoryStorage {
    async fn is_available(&self) -> bool {
        // Memory storage is always available
        true
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        // No initialization needed for memory storage
        Ok(())
    }

    fn backend_type(&self) -> &str {
        "memory"
    }
}

#[async_trait]
impl ConversationStorage for MemoryStorage {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        let mut conversations = self.conversations.write().await;

        // Check if this is a new conversation
        let is_new = !conversations.contains_key(&conversation.id);

        if is_new {
            // Check size limit
            if !self.can_add_item().await {
                return Err(StorageError::InvalidOperation(format!(
                    "Memory storage limit reached (max: {})",
                    self.max_size
                )));
            }
        }

        conversations.insert(conversation.id, conversation.clone());

        if is_new {
            self.increment_size().await;

            // Create initial metadata entry
            let mut metadata_map = self.conversation_metadata.write().await;
            metadata_map.insert(
                conversation.id,
                ConversationMetadata {
                    id: Uuid::new_v4(),
                    conversation_id: conversation.id,
                    title: None,
                    message_count: 0,
                    last_message_at: conversation.created_at,
                    created_at: conversation.created_at,
                    updated_at: conversation.updated_at,
                },
            );
        }

        Ok(conversation.id)
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        let conversations = self.conversations.read().await;
        Ok(conversations.get(&id).cloned())
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        let conversations = self.conversations.read().await;
        let messages = self.messages.read().await;
        let metadata_map = self.conversation_metadata.read().await;

        let mut result: Vec<ConversationMetadata> = conversations
            .values()
            .filter(|c| c.user_id == user_id)
            .map(|c| {
                let message_count = messages.get(&c.id).map(|msgs| msgs.len()).unwrap_or(0);

                // Get last message timestamp
                let last_message_at = messages
                    .get(&c.id)
                    .and_then(|msgs| msgs.last())
                    .map(|msg| msg.created_at)
                    .unwrap_or(c.created_at);

                // Get stored metadata for title, or create new
                let stored_metadata = metadata_map.get(&c.id);

                ConversationMetadata {
                    id: stored_metadata.map(|m| m.id).unwrap_or_else(Uuid::new_v4),
                    conversation_id: c.id,
                    title: stored_metadata.and_then(|m| m.title.clone()),
                    created_at: c.created_at,
                    updated_at: c.updated_at,
                    message_count,
                    last_message_at,
                }
            })
            .collect();

        // Sort by updated_at descending (most recent first)
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(result)
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        let mut conversations = self.conversations.write().await;
        let mut messages = self.messages.write().await;
        let mut metadata_map = self.conversation_metadata.write().await;

        if conversations.remove(&id).is_some() {
            messages.remove(&id);
            metadata_map.remove(&id);
            self.decrement_size().await;
        }

        Ok(())
    }
}

#[async_trait]
impl MessageStorage for MemoryStorage {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        // Verify conversation exists
        let conversations = self.conversations.read().await;
        if !conversations.contains_key(&message.conversation_id) {
            return Err(StorageError::NotFound(format!(
                "Conversation {} not found",
                message.conversation_id
            )));
        }
        drop(conversations);

        let mut messages = self.messages.write().await;

        // Get or create message list for this conversation
        let conversation_messages = messages
            .entry(message.conversation_id)
            .or_insert_with(Vec::new);

        // Check if message already exists and update, or add new
        if let Some(existing) = conversation_messages
            .iter_mut()
            .find(|m| m.id == message.id)
        {
            *existing = message.clone();
        } else {
            conversation_messages.push(message.clone());
        }

        // Sort messages by created_at to maintain order
        conversation_messages.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Update metadata with new message count and last message time
        let mut metadata_map = self.conversation_metadata.write().await;
        if let Some(metadata) = metadata_map.get_mut(&message.conversation_id) {
            metadata.message_count = conversation_messages.len();
            metadata.last_message_at = message.created_at;
            metadata.updated_at = message.created_at;
        }

        Ok(message.id)
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        let messages = self.messages.read().await;
        Ok(messages.get(&conversation_id).cloned().unwrap_or_default())
    }
}

#[async_trait]
impl UserStorage for MemoryStorage {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        let mut users = self.users.write().await;
        users.insert(user.id, user.clone());
        Ok(user.id)
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        let users = self.users.read().await;
        Ok(users.get(&id).cloned())
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        let users = self.users.read().await;
        let user = users
            .iter()
            .find(|x| x.1.name == name.to_string());

        match user {
            Some(u) => Ok(Some(u.1.clone())),
            None => Err(StorageError::NotFound(format!(
                "MemoryStorage: Couldn't find user: '{name}'"
            ))),
        }
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id, session.clone());
        Ok(session.id)
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(&id).cloned())
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let sessions = self.sessions.read().await;
        let session = sessions.iter().find(|x| x.1.token == token);

        match session {
            Some(s) => Ok(Some(s.1.clone())),
            None => Err(StorageError::NotFound(format!(
                "MemoryStorage: Couldn't find session token: '{token}'"
            ))),
        }
    }

    // update_session_activity and revoke_session remain the same
    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id) {
            Some(s) => {
                s.last_active_at = last_active_at;
                if let Some(expires_at) = new_expiration {
                    s.expires_at = expires_at;
                }
                Ok(())
            }
            None => Err(StorageError::NotFound(format!(
                "MemoryStorage: Couldn't find session: '{session_id}'"
            ))),
        }
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id) {
            Some(s) => {
                s.revoked_at = Some(Utc::now());
                Ok(())
            }
            None => Err(StorageError::NotFound(format!(
                "MemoryStorage: Couldn't find session: '{session_id}'"
            ))),
        }
    }
    
    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;
        match sessions.get_mut(&session_id) {
            Some(s) => {
                s.csrf_token = new_csrf_token;
                s.csrf_rotation_count = rotation_count;
                s.csrf_last_rotated_at = rotated_at;
                Ok(())
            }
            None => Err(StorageError::NotFound(format!(
                "MemoryStorage: Couldn't find session: '{session_id}'"
            ))),
        }
    }
}

impl CharacterStorage for MemoryStorage {}

#[async_trait]
impl TemplateStorage for MemoryStorage {
    async fn save_template(&self, template: &Template) -> Result<Uuid, StorageError> {
        let mut templates = self.templates.write().await;

        // Check if this is a new template
        let is_new = !templates.contains_key(&template.id);

        if is_new {
            // Check size limit
            if !self.can_add_item().await {
                return Err(StorageError::InvalidOperation(format!(
                    "Memory storage limit reached (max: {})",
                    self.max_size
                )));
            }
        }

        templates.insert(template.id, template.clone());

        if is_new {
            self.increment_size().await;
        }

        Ok(template.id)
    }

    async fn get_template(&self, id: Uuid) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.get(&id).cloned())
    }

    async fn get_template_owned(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Template>, StorageError> {
        let templates = self.templates.read().await;
        
        match templates.get(&id) {
            Some(template) if template.owner_id == owner_id => Ok(Some(template.clone())),
            Some(_) => Err(StorageError::PermissionDenied(
                "Template not owned by user".to_string(),
            )),
            None => Ok(None),
        }
    }

    async fn list_templates(&self, owner_id: Uuid) -> Result<Vec<TemplateMetadata>, StorageError> {
        let templates = self.templates.read().await;

        let mut result: Vec<TemplateMetadata> = templates
            .values()
            .filter(|t| t.owner_id == owner_id)
            .map(|t| {
                // Parse nodes_json to count nodes
                let node_count = if let Ok(nodes) =
                    serde_json::from_str::<serde_json::Value>(&t.nodes_json)
                {
                    nodes.as_array().map(|arr| arr.len()).unwrap_or(0)
                } else {
                    0
                };

                TemplateMetadata {
                    id: t.id,
                    owner_id: t.owner_id,
                    name: t.name.clone(),
                    description: t.description.clone(),
                    node_count,
                    version: t.version,
                    created_at: t.created_at,
                    updated_at: t.updated_at,
                }
            })
            .collect();

        // Sort by created_at descending (most recent first)
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(result)
    }

    async fn delete_template(&self, id: Uuid, owner_id: Uuid) -> Result<(), StorageError> {
        let mut templates = self.templates.write().await;

        // First verify ownership
        if let Some(template) = templates.get(&id) {
            if template.owner_id != owner_id {
                return Err(StorageError::PermissionDenied(
                    "Cannot delete template not owned by user".to_string(),
                ));
            }
        } else {
            return Err(StorageError::NotFound(format!(
                "Template {id} not found"
            )));
        }

        if templates.remove(&id).is_some() {
            self.decrement_size().await;
        }

        Ok(())
    }

    async fn template_exists(&self, id: Uuid) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;
        Ok(templates.contains_key(&id))
    }

    async fn template_name_exists(
        &self,
        owner_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, StorageError> {
        let templates = self.templates.read().await;

        for template in templates.values() {
            if template.owner_id == owner_id
                && template.name == name
                && exclude_id.map_or(true, |exclude| template.id != exclude)
            {
                return Ok(true);
            }
        }

        Ok(false)
    }
}

impl_storage_capabilities!(MemoryStorage, {
    supports_conversations: true,
    supports_messages: true,
    supports_users: true,
    supports_sessions: true,
    supports_characters: false,
    supports_templates: true
});

// Implement Clone for MemoryStorage to allow sharing
impl Clone for MemoryStorage {
    fn clone(&self) -> Self {
        Self {
            conversations: Arc::clone(&self.conversations),
            conversation_metadata: Arc::clone(&self.conversation_metadata),
            messages: Arc::clone(&self.messages),
            users: Arc::clone(&self.users),
            sessions: Arc::clone(&self.sessions),
            templates: Arc::clone(&self.templates),
            max_size: self.max_size,
            current_size: Arc::clone(&self.current_size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_memory_storage_basic() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conversation = ConversationBuilder::new()
            .id(convo_id)
            .user_id(user_id)
            .build();

        storage.save_conversation(&conversation).await.unwrap();
        let retrieved = storage.get_conversation(convo_id).await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, convo_id);
    }

    #[tokio::test]
    async fn test_memory_storage_size_limit() {
        let storage = MemoryStorage::new(2);

        let conv1 = ConversationBuilder::new().id(Uuid::new_v4()).build();
        let conv2 = ConversationBuilder::new().id(Uuid::new_v4()).build();
        let conv3 = ConversationBuilder::new().id(Uuid::new_v4()).build();

        storage.save_conversation(&conv1).await.unwrap();
        storage.save_conversation(&conv2).await.unwrap();

        // Third should fail
        let result = storage.save_conversation(&conv3).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_memory_storage_unlimited() {
        let storage = MemoryStorage::new(0); // Unlimited

        for _ in 0..1000 {
            let conv = ConversationBuilder::new().id(Uuid::new_v4()).build();
            storage.save_conversation(&conv).await.unwrap();
        }

        let stats = storage.stats().await;
        assert_eq!(stats.conversation_count, 1000);
    }

    #[tokio::test]
    async fn test_memory_storage_messages() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();
        let conversation = ConversationBuilder::new().id(convo_id).build();
        storage.save_conversation(&conversation).await.unwrap();

        let msg1_id = Uuid::new_v4();
        let msg1 = MessageBuilder::new(convo_id, None)
            .id(msg1_id)
            .content("First")
            .build();

        let msg2 = MessageBuilder::new(convo_id, Some(msg1_id))
            .id(Uuid::new_v4())
            .content("Second")
            .build();

        storage.save_message(&msg1).await.unwrap();
        storage.save_message(&msg2).await.unwrap();

        let messages = storage.get_messages(convo_id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].content, "First");
        assert_eq!(messages[1].content, "Second");
    }

    #[tokio::test]
    async fn test_memory_storage_message_ordering() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();

        let conversation = ConversationBuilder::new().id(convo_id).build();
        storage.save_conversation(&conversation).await.unwrap();

        let mut last_id = None;
        // Add messages in reverse order
        for i in (0..10).rev() {
            let mes_id = Uuid::new_v4();
            let mut msg = MessageBuilder::new(convo_id, last_id)
                .id(mes_id)
                .content(&format!("Message {}", i))
                .build();

            last_id = Some(mes_id);

            // Manually set created_at to ensure ordering
            msg.created_at = chrono::Utc::now() + chrono::Duration::seconds(i);

            storage.save_message(&msg).await.unwrap();
        }

        let messages = storage.get_messages(convo_id).await.unwrap();

        // Should be sorted by created_at
        for (i, msg) in messages.iter().enumerate() {
            assert_eq!(msg.content, format!("Message {}", i));
        }
    }

    #[tokio::test]
    async fn test_memory_storage_message_update() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();

        let conversation = ConversationBuilder::new().id(convo_id).build();
        storage.save_conversation(&conversation).await.unwrap();

        let msg = MessageBuilder::new(convo_id, None)
            .id(Uuid::new_v4())
            .content("Original")
            .build();

        storage.save_message(&msg).await.unwrap();

        // Update the message
        let mut updated = msg.clone();
        updated.content = "Updated".to_string();
        storage.save_message(&updated).await.unwrap();

        let messages = storage.get_messages(convo_id).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Updated");
    }

    #[tokio::test]
    async fn test_memory_storage_message_without_conversation() {
        let storage = MemoryStorage::new(100);

        let msg = MessageBuilder::new(Uuid::new_v4(), None) // non-existent
            .build();

        let result = storage.save_message(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_memory_storage_list_conversations() {
        let storage = MemoryStorage::new(100);

        let convo_id_1 = Uuid::new_v4();
        let convo_id_2 = Uuid::new_v4();
        let convo_id_3 = Uuid::new_v4();

        let user_id_1 = Uuid::new_v4();
        let user_id_2 = Uuid::new_v4();

        let conv1 = ConversationBuilder::new()
            .id(convo_id_1)
            .user_id(user_id_1)
            .build();

        let conv2 = ConversationBuilder::new()
            .id(convo_id_2)
            .user_id(user_id_1)
            .build();

        let conv3 = ConversationBuilder::new()
            .id(convo_id_3)
            .user_id(user_id_2)
            .build();

        storage.save_conversation(&conv1).await.unwrap();
        storage.save_conversation(&conv2).await.unwrap();
        storage.save_conversation(&conv3).await.unwrap();

        let user1_convs = storage.list_conversations(user_id_1).await.unwrap();
        assert_eq!(user1_convs.len(), 2);

        let user2_convs = storage.list_conversations(user_id_2).await.unwrap();
        assert_eq!(user2_convs.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_storage_delete() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();

        let conversation = ConversationBuilder::new().id(convo_id).build();
        storage.save_conversation(&conversation).await.unwrap();

        let msg = MessageBuilder::new(convo_id, None).build();
        storage.save_message(&msg).await.unwrap();

        storage.delete_conversation(convo_id).await.unwrap();

        let retrieved = storage.get_conversation(convo_id).await.unwrap();
        assert!(retrieved.is_none());

        let messages = storage.get_messages(convo_id).await.unwrap();
        assert_eq!(messages.len(), 0);
    }

    #[tokio::test]
    async fn test_memory_storage_clear() {
        let storage = MemoryStorage::new(100);

        for _ in 0..10 {
            let conv = ConversationBuilder::new().id(Uuid::new_v4()).build();
            storage.save_conversation(&conv).await.unwrap();
        }

        let stats_before = storage.stats().await;
        assert_eq!(stats_before.conversation_count, 10);

        storage.clear().await;

        let stats_after = storage.stats().await;
        assert_eq!(stats_after.conversation_count, 0);
        assert_eq!(stats_after.current_size, 0);
    }

    #[tokio::test]
    async fn test_memory_storage_stats() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();

        let conv = ConversationBuilder::new().id(convo_id).build();
        storage.save_conversation(&conv).await.unwrap();

        let user = UserBuilder::new().id(Uuid::new_v4()).build();
        storage.save_user(&user).await.unwrap();

        let msg1 = MessageBuilder::new(convo_id, None).build();
        let msg2 = MessageBuilder::new(convo_id, Some(msg1.id)).build();
        storage.save_message(&msg1).await.unwrap();
        storage.save_message(&msg2).await.unwrap();

        let stats = storage.stats().await;
        assert_eq!(stats.conversation_count, 1);
        assert_eq!(stats.user_count, 1);
        assert_eq!(stats.message_count, 2);
        assert_eq!(stats.max_size, 100);
    }

    #[tokio::test]
    async fn test_memory_storage_users() {
        let storage = MemoryStorage::new(100);
        let user_id = Uuid::new_v4();

        let user = UserBuilder::new()
            .id(user_id)
            .email("test@example.com")
            .build();

        storage.save_user(&user).await.unwrap();

        let retrieved = storage.get_user_by_id(user_id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(
            retrieved.unwrap().email,
            Some("test@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn test_memory_storage_concurrent_access() {
        let storage = MemoryStorage::new(100);
        let storage_clone1 = storage.clone();
        let storage_clone2 = storage.clone();

        let handle1 = tokio::spawn(async move {
            for _ in 0..50 {
                let conv = ConversationBuilder::new().id(Uuid::new_v4()).build();
                storage_clone1.save_conversation(&conv).await.unwrap();
            }
        });

        let handle2 = tokio::spawn(async move {
            for _ in 0..50 {
                let conv = ConversationBuilder::new().id(Uuid::new_v4()).build();
                storage_clone2.save_conversation(&conv).await.unwrap();
            }
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        // Both tasks should have succeeded, but we hit the limit
        let stats = storage.stats().await;
        assert_eq!(stats.conversation_count, 100);
    }

    #[tokio::test]
    async fn test_memory_storage_metadata_tracking() {
        let storage = MemoryStorage::new(100);
        let convo_id = Uuid::new_v4();
        let user_id = Uuid::new_v4();

        let conversation = ConversationBuilder::new()
            .id(convo_id)
            .user_id(user_id)
            .build();

        storage.save_conversation(&conversation).await.unwrap();

        // Add some messages
        let msg1 = MessageBuilder::new(convo_id, None).build();
        let msg2 = MessageBuilder::new(convo_id, Some(msg1.id)).build();
        storage.save_message(&msg1).await.unwrap();
        storage.save_message(&msg2).await.unwrap();

        // List conversations and verify metadata
        let metadata_list = storage.list_conversations(user_id).await.unwrap();
        assert_eq!(metadata_list.len(), 1);

        let metadata = &metadata_list[0];
        assert_eq!(metadata.conversation_id, convo_id);
        assert_eq!(metadata.message_count, 2);
        assert!(metadata.last_message_at >= msg1.created_at);
    }
}
