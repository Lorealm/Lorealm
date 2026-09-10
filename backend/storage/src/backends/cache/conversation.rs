use crate::ConversationStorage;
use crate::backends::CachedStorage;
use crate::error::StorageError;
use crate::models::{
    Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake,
};
use async_trait::async_trait;

#[async_trait]
impl ConversationStorage for CachedStorage {
    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        self.backend.list_conversations(user_id).await
    }

    async fn get_conversation(&self, id: Snowflake) -> Result<Option<Conversation>, StorageError> {
        let key = self.key("conversation", &id.to_string());
        self.get_through(&key, self.backend.get_conversation(id))
            .await
    }

    async fn create_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Snowflake, StorageError> {
        self.backend.create_conversation(conversation).await
    }

    async fn update_conversation(
        &self,
        id: Snowflake,
        title: Option<&str>,
        active_leaf_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.backend
            .update_conversation(id, title, active_leaf_id)
            .await?;
        self.cache_del(&self.key("conversation", &id.to_string()))
            .await;
        Ok(())
    }

    async fn set_author_note(&self, id: Snowflake, note: &str) -> Result<(), StorageError> {
        self.backend.set_author_note(id, note).await?;
        self.cache_del(&self.key("conversation", &id.to_string()))
            .await;
        Ok(())
    }

    async fn delete_conversation(&self, id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_conversation(id).await?;
        if deleted {
            self.cache_del(&self.key("conversation", &id.to_string()))
                .await;
        }
        Ok(deleted)
    }

    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError> {
        let id = self.backend.add_message(message).await?;
        // The conversation's updated_at changed.
        self.cache_del(&self.key("conversation", &message.conversation_id.to_string()))
            .await;
        Ok(id)
    }

    async fn set_active_leaf(
        &self,
        conversation_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        self.backend.set_active_leaf(conversation_id, leaf).await
    }

    async fn append_and_advance(
        &self,
        message: &ConversationMessage,
    ) -> Result<Snowflake, StorageError> {
        let id = self.backend.append_and_advance(message).await?;
        // The conversation's active_leaf_id and updated_at both changed.
        self.cache_del(&self.key("conversation", &message.conversation_id.to_string()))
            .await;
        Ok(id)
    }

    async fn get_message(
        &self,
        id: Snowflake,
    ) -> Result<Option<ConversationMessage>, StorageError> {
        let key = self.key("message", &id.to_string());
        self.get_through(&key, self.backend.get_message(id)).await
    }

    async fn list_messages(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        self.backend.list_messages(conversation_id).await
    }

    async fn get_children(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        self.backend.get_children(conversation_id, parent).await
    }

    async fn get_active_path(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        self.backend.get_active_path(conversation_id).await
    }

    async fn get_path_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        self.backend.get_path_to(conversation_id, message_id).await
    }

    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError> {
        self.backend.update_message(id, edit).await?;
        self.cache_del(&self.key("message", &id.to_string())).await;
        Ok(())
    }

    async fn delete_message(&self, id: Snowflake) -> Result<bool, StorageError> {
        let deleted = self.backend.delete_message(id).await?;
        if deleted {
            self.cache_del(&self.key("message", &id.to_string())).await;
        }
        Ok(deleted)
    }
}
