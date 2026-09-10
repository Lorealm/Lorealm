use crate::error::StorageError;
use crate::models::{
    Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake,
};
use async_trait::async_trait;

#[async_trait]
pub trait ConversationStorage: Send + Sync {
    // ----- conversation rows -----

    /// List a user's conversations, most-recently-updated first.
    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError>;

    /// Load a conversation's metadata row.
    async fn get_conversation(&self, id: Snowflake) -> Result<Option<Conversation>, StorageError>;

    /// Create a conversation row.
    async fn create_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Snowflake, StorageError>;

    /// Update mutable metadata
    async fn update_conversation(
        &self,
        id: Snowflake,
        title: Option<&str>,
        active_leaf_id: Option<Snowflake>,
    ) -> Result<(), StorageError>;

    /// Replace a conversation's author's note.
    async fn set_author_note(&self, id: Snowflake, note: &str) -> Result<(), StorageError>;

    /// Hard-delete a conversation
    async fn delete_conversation(&self, id: Snowflake) -> Result<bool, StorageError>;

    /// Append a message.
    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError>;

    /// Set the active leaf node of the given conversation
    async fn set_active_leaf(
        &self,
        conversation_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<(), StorageError>;

    /// Append a message AND set it as the conversation's active leaf
    async fn append_and_advance(
        &self,
        message: &ConversationMessage,
    ) -> Result<Snowflake, StorageError>;

    /// Load a single message.
    async fn get_message(&self, id: Snowflake)
    -> Result<Option<ConversationMessage>, StorageError>;

    /// Load the full message tree of the given conversation
    async fn list_messages(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// Ordered children of a node. Only immediate children.
    async fn get_children(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// The linear path from root to the active leaf
    async fn get_active_path(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// The linear path root -> `message_id`
    async fn get_path_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// Edit a message's mutable fields
    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError>;

    /// Delete a message and its entire subtree
    async fn delete_message(&self, id: Snowflake) -> Result<bool, StorageError>;
}
