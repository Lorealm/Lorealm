//! Conversation storage trait.
//!
//! Unlike `CharacterStorage`, conversations are NOT coarse-grained aggregates.
//! A conversation's messages form an unbounded tree (branches + swipes), so
//! writing the whole thing per turn is untenable. Instead the conversation row
//! is a thin metadata record and messages are appended/edited/deleted
//! individually. Subtree deletes ride on the schema's ON DELETE CASCADE
//! (`conversation_messages.parent`); clearing a dangling active leaf rides on
//! the ON DELETE SET NULL on `fk_conversations_active_leaf`.

use crate::error::StorageError;
use crate::models::{
    Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake,
};
use async_trait::async_trait;

#[async_trait]
pub trait ConversationStorage: Send + Sync {
    // ----- conversation rows -----

    /// List a user's conversations, most-recently-updated first.
    /// Backed by `idx_conversations_user_recent`. Summaries only — no messages.
    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError>;

    /// Load a conversation's metadata row. `None` if absent. Does not load
    /// messages — use `list_messages` / `get_active_path` for those.
    async fn get_conversation(&self, id: Snowflake) -> Result<Option<Conversation>, StorageError>;

    /// Create a conversation row. `active_leaf_id` is expected to be `None` at
    /// creation (no messages yet). Caller generates `conversation.id`.
    async fn create_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Snowflake, StorageError>;

    /// Update mutable metadata (`title`, `active_leaf_id`) and bump
    /// `updated_at`. `active_leaf_id` must reference a message in this
    /// conversation or be `None`.
    async fn update_conversation(
        &self,
        id: Snowflake,
        title: Option<&str>,
        active_leaf_id: Option<Snowflake>,
    ) -> Result<(), StorageError>;

    /// Hard-delete a conversation and (via cascade) all its messages.
    /// Returns `true` if a row was deleted.
    async fn delete_conversation(&self, id: Snowflake) -> Result<bool, StorageError>;

    // ----- messages -----

    /// Append a message. Caller generates `message.id` and sets `parent`
    /// (`None` for the root) and `order` among its siblings. Bumps the parent
    /// conversation's `updated_at`. Does NOT move `active_leaf_id`.
    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError>;

    async fn set_active_leaf(
        &self,
        conversation_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<(), StorageError>;

    /// Append a message AND set it as the conversation's active leaf, in one
    /// transaction. This is the normal-turn path (reply / regenerate), so it's
    /// worth a dedicated atomic op rather than `add_message` + a separate
    /// `update_conversation` that could tear.
    async fn append_and_advance(
        &self,
        message: &ConversationMessage,
    ) -> Result<Snowflake, StorageError>;

    /// Load a single message. `None` if absent.
    async fn get_message(&self, id: Snowflake)
    -> Result<Option<ConversationMessage>, StorageError>;

    /// Load every message in a conversation (the full tree, unordered across
    /// branches). Backed by `idx_messages_conversation`. Caller reconstructs
    /// structure from `parent` / `order`.
    async fn list_messages(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// Ordered children of a node — the sibling set for swipe / branch nav.
    /// Pass `parent = None` for the conversation's roots. Ordered by `"order"`;
    /// backed by `idx_messages_parent_order`.
    async fn get_children(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// The linear path root → `active_leaf_id`, in order: the messages you
    /// actually send to the model. Empty if there's no active leaf. Implement
    /// as a recursive CTE walking `parent` up from the leaf — no per-message
    /// round trips.
    async fn get_active_path(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError>;

    /// Edit a message's mutable fields (`content`, `metadata`) in place and
    /// bump `updated_at`. Does not branch — for a non-destructive edit,
    /// `add_message` a sibling instead.
    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError>;

    /// Delete a message and (via cascade) its entire subtree. Returns `true`
    /// if a row was deleted. If the active leaf was inside the deleted subtree
    /// the DB sets `active_leaf_id` to NULL; the caller should pick a new leaf.
    async fn delete_message(&self, id: Snowflake) -> Result<bool, StorageError>;
}
