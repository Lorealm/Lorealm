use std::collections::HashSet;

use async_trait::async_trait;
use chrono::Utc;

use crate::{
    ConversationStorage, StorageError,
    backends::MemoryStorage,
    models::{Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake},
};

#[async_trait]
impl ConversationStorage for MemoryStorage {
    // ----- conversation rows -----

    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        let conversations = self.conversations.read().await;
        let messages = self.messages.read().await;

        let mut result: Vec<ConversationMetadata> = conversations
            .values()
            .filter(|c| c.user_id == user_id)
            .map(|c| ConversationMetadata {
                id: c.id,
                character_id: c.character_id,
                char_version_id: c.char_version_id,
                title: c.title.clone(),
                length: Self::active_path_len(&messages, c.active_leaf_id),
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect();

        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(result)
    }

    async fn get_conversation(&self, id: Snowflake) -> Result<Option<Conversation>, StorageError> {
        Ok(self.conversations.read().await.get(&id).cloned())
    }

    async fn create_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Snowflake, StorageError> {
        self.conversations
            .write()
            .await
            .insert(conversation.id, conversation.clone());
        Ok(conversation.id)
    }

    async fn update_conversation(
        &self,
        id: Snowflake,
        title: Option<&str>,
        active_leaf_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        let mut conversations = self.conversations.write().await;
        let conv = conversations
            .get_mut(&id)
            .ok_or_else(|| StorageError::NotFound(format!("Conversation {id} not found")))?;

        // Direct set (NULL when None), matching the Postgres impl.
        conv.title = title.map(|s| s.to_string());
        conv.active_leaf_id = active_leaf_id;
        conv.updated_at = Utc::now();
        Ok(())
    }

    async fn delete_conversation(&self, id: Snowflake) -> Result<bool, StorageError> {
        let removed = self.conversations.write().await.remove(&id).is_some();
        if removed {
            // Cascade: drop every message in this conversation.
            self.messages
                .write()
                .await
                .retain(|_, m| m.conversation_id != id);
        }
        Ok(removed)
    }

    // ----- messages -----

    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError> {
        {
            let conversations = self.conversations.read().await;
            if !conversations.contains_key(&message.conversation_id) {
                return Err(StorageError::NotFound(format!(
                    "Conversation {} not found",
                    message.conversation_id
                )));
            }
        }

        self.messages
            .write()
            .await
            .insert(message.id, message.clone());

        if let Some(conv) = self
            .conversations
            .write()
            .await
            .get_mut(&message.conversation_id)
        {
            conv.updated_at = Utc::now();
        }

        Ok(message.id)
    }

    async fn set_active_leaf(
        &self,
        conversation_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        if let Some(conv) = self.conversations.write().await.get_mut(&conversation_id) {
            conv.active_leaf_id = leaf;
            conv.updated_at = Utc::now();
        };
        Ok(())
    }

    async fn append_and_advance(
        &self,
        message: &ConversationMessage,
    ) -> Result<Snowflake, StorageError> {
        {
            let conversations = self.conversations.read().await;
            if !conversations.contains_key(&message.conversation_id) {
                return Err(StorageError::NotFound(format!(
                    "Conversation {} not found",
                    message.conversation_id
                )));
            }
        }

        self.messages
            .write()
            .await
            .insert(message.id, message.clone());

        if let Some(conv) = self
            .conversations
            .write()
            .await
            .get_mut(&message.conversation_id)
        {
            conv.active_leaf_id = Some(message.id);
            conv.updated_at = Utc::now();
        }

        Ok(message.id)
    }

    async fn get_message(
        &self,
        id: Snowflake,
    ) -> Result<Option<ConversationMessage>, StorageError> {
        Ok(self.messages.read().await.get(&id).cloned())
    }

    async fn list_messages(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        let messages = self.messages.read().await;
        Ok(messages
            .values()
            .filter(|m| m.conversation_id == conversation_id)
            .cloned()
            .collect())
    }

    async fn get_children(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        let messages = self.messages.read().await;
        let mut children: Vec<ConversationMessage> = messages
            .values()
            .filter(|m| m.conversation_id == conversation_id && m.parent == parent)
            .cloned()
            .collect();

        children.sort_by_key(|m| m.order);
        Ok(children)
    }

    async fn get_active_path(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        let leaf = {
            let conversations = self.conversations.read().await;
            match conversations.get(&conversation_id) {
                Some(c) => c.active_leaf_id,
                None => return Ok(Vec::new()),
            }
        };

        let messages = self.messages.read().await;
        let mut path = Vec::new();
        let mut current = leaf;
        while let Some(id) = current {
            match messages.get(&id) {
                Some(m) => {
                    current = m.parent;
                    path.push(m.clone());
                }
                None => break,
            }
        }

        path.reverse(); // collected leaf -> root; return root -> leaf
        Ok(path)
    }

    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError> {
        let mut messages = self.messages.write().await;
        let message = messages
            .get_mut(&id)
            .ok_or_else(|| StorageError::NotFound(format!("Message {id} not found")))?;

        if let Some(content) = &edit.content {
            message.content = content.clone();
        }
        if let Some(metadata) = &edit.metadata {
            message.metadata = metadata.clone();
        }
        message.updated_at = Utc::now();
        Ok(())
    }

    async fn delete_message(&self, id: Snowflake) -> Result<bool, StorageError> {
        let conversation_id;
        let deleted: HashSet<Snowflake>;
        {
            let mut messages = self.messages.write().await;
            let Some(target) = messages.get(&id) else {
                return Ok(false);
            };
            conversation_id = target.conversation_id;

            let subtree = Self::collect_subtree(&messages, id);
            for m in &subtree {
                messages.remove(m);
            }
            deleted = subtree.into_iter().collect();
        }

        // SET NULL: if the active leaf was inside the deleted subtree, clear it.
        let mut conversations = self.conversations.write().await;
        if let Some(conv) = conversations.get_mut(&conversation_id) {
            if conv
                .active_leaf_id
                .is_some_and(|leaf| deleted.contains(&leaf))
            {
                conv.active_leaf_id = None;
            }
        }

        Ok(true)
    }
}
