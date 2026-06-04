//! Conversation + message storage for Postgres.
//!
//! Mirrors the `UserStorage` impl style: `sqlx` without macros, `query_as`
//! against `FromRow`-deriving models. The message tree's structural invariants
//! (`parent`, `"order"`, `active_leaf_id`) are maintained here; subtree deletes
//! ride on the schema's ON DELETE CASCADE, and a cleared active leaf rides on
//! the ON DELETE SET NULL on `fk_conversations_active_leaf`.
//!
//! `"order"` is a SQL reserved word and is quoted in every query that touches
//! it. `MessageRole` is assumed to encode/decode as TEXT to match the column's
//! CHECK constraint.

use crate::ConversationStorage;
use crate::error::StorageError;
use crate::models::{
    Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake,
};
use async_trait::async_trait;

use super::PostgresStorage;

#[async_trait]
impl ConversationStorage for PostgresStorage {
    // ----- conversation rows -----

    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        // `length` = number of messages on each conversation's active path
        // (root -> active_leaf_id), computed by walking `parent` up from the
        // leaf. Conversations with a NULL active leaf never seed the CTE and
        // fall through the LEFT JOIN to COALESCE(0).
        let rows = sqlx::query_as::<_, ConversationMetadata>(
            r#"
            WITH RECURSIVE active_paths AS (
                SELECT c.id AS conversation_id, m.parent
                FROM conversations c
                JOIN conversation_messages m ON m.id = c.active_leaf_id
                WHERE c.user_id = $1

                UNION ALL

                SELECT ap.conversation_id, m.parent
                FROM active_paths ap
                JOIN conversation_messages m ON m.id = ap.parent
            )
            SELECT
                c.id,
                c.character_id,
                c.char_version_id,
                c.title,
                c.created_at,
                c.updated_at,
                COALESCE(lengths.length, 0) AS length
            FROM conversations c
            LEFT JOIN (
                SELECT conversation_id, COUNT(*) AS length
                FROM active_paths
                GROUP BY conversation_id
            ) lengths ON lengths.conversation_id = c.id
            WHERE c.user_id = $1
            ORDER BY c.updated_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }

    async fn get_conversation(&self, id: Snowflake) -> Result<Option<Conversation>, StorageError> {
        let conversation =
            sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await?;

        Ok(conversation)
    }

    async fn create_conversation(
        &self,
        conversation: &Conversation,
    ) -> Result<Snowflake, StorageError> {
        // `active_leaf_id` is expected to be None at creation. Caller owns the id.
        sqlx::query(
            "INSERT INTO conversations
                 (id, user_id, character_id, char_version_id, active_leaf_id, title)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(conversation.id)
        .bind(conversation.user_id)
        .bind(conversation.character_id)
        .bind(conversation.char_version_id)
        .bind(conversation.active_leaf_id)
        .bind(conversation.title.as_deref())
        .execute(&self.pool)
        .await?;

        Ok(conversation.id)
    }

    async fn update_conversation(
        &self,
        id: Snowflake,
        title: Option<&str>,
        active_leaf_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        // Both fields are set directly (NULL when None) rather than
        // "update-if-Some". The common leaf-only move goes through
        // `append_and_advance`; this method is for explicit metadata writes.
        let result = sqlx::query(
            "UPDATE conversations
                 SET title = $2, active_leaf_id = $3, updated_at = now()
                 WHERE id = $1",
        )
        .bind(id)
        .bind(title)
        .bind(active_leaf_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Conversation {id} not found"
            )));
        }

        Ok(())
    }

    async fn delete_conversation(&self, id: Snowflake) -> Result<bool, StorageError> {
        // Cascades to conversation_messages via ON DELETE CASCADE.
        let result = sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ----- messages -----

    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError> {
        // Insert + bump the parent conversation's updated_at, atomically.
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO conversation_messages
                (id, conversation_id, parent, role, content, "order", metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(message.id)
        .bind(message.conversation_id)
        .bind(message.parent)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.order)
        .bind(&message.metadata)
        .execute(&mut *tx)
        .await?;

        sqlx::query("UPDATE conversations SET updated_at = now() WHERE id = $1")
            .bind(message.conversation_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(message.id)
    }

    async fn set_active_leaf(
        &self,
        conversation_id: Snowflake,
        leaf: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        let result = sqlx::query(
            "UPDATE conversations SET active_leaf_id = $2, updated_at = now() WHERE id = $1",
        )
        .bind(conversation_id)
        .bind(leaf)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!(
                "Conversation {conversation_id} not found"
            )));
        }
        Ok(())
    }

    async fn append_and_advance(
        &self,
        message: &ConversationMessage,
    ) -> Result<Snowflake, StorageError> {
        // The normal-turn path: insert the message AND make it the active leaf
        // in one transaction so the two can never tear apart.
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO conversation_messages
                (id, conversation_id, parent, role, content, "order", metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(message.id)
        .bind(message.conversation_id)
        .bind(message.parent)
        .bind(&message.role)
        .bind(&message.content)
        .bind(message.order)
        .bind(&message.metadata)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            "UPDATE conversations
                 SET active_leaf_id = $1, updated_at = now()
                 WHERE id = $2",
        )
        .bind(message.id)
        .bind(message.conversation_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(message.id)
    }

    async fn get_message(
        &self,
        id: Snowflake,
    ) -> Result<Option<ConversationMessage>, StorageError> {
        let message = sqlx::query_as::<_, ConversationMessage>(
            "SELECT * FROM conversation_messages WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(message)
    }

    async fn list_messages(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        // Full tree, unordered across branches; caller rebuilds structure from
        // parent / "order". Backed by idx_messages_conversation.
        let messages = sqlx::query_as::<_, ConversationMessage>(
            "SELECT * FROM conversation_messages WHERE conversation_id = $1",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(messages)
    }

    async fn get_children(
        &self,
        conversation_id: Snowflake,
        parent: Option<Snowflake>,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        // `IS NOT DISTINCT FROM` so a None parent matches the conversation's
        // roots (parent IS NULL) instead of matching nothing. conversation_id
        // scopes the roots; for non-NULL parents idx_messages_parent_order
        // carries the lookup + sort.
        let children = sqlx::query_as::<_, ConversationMessage>(
            r#"
            SELECT * FROM conversation_messages
            WHERE conversation_id = $1 AND parent IS NOT DISTINCT FROM $2
            ORDER BY "order"
            "#,
        )
        .bind(conversation_id)
        .bind(parent)
        .fetch_all(&self.pool)
        .await?;

        Ok(children)
    }

    async fn get_active_path(
        &self,
        conversation_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        // Walk parent up from the active leaf, then return root -> leaf by
        // ordering on the depth counter descending (seed = leaf = depth 0).
        let path = sqlx::query_as::<_, ConversationMessage>(
            r#"
            WITH RECURSIVE active_path AS (
                SELECT m.*, 0 AS depth
                FROM conversation_messages m
                JOIN conversations c ON c.active_leaf_id = m.id
                WHERE c.id = $1

                UNION ALL

                SELECT m.*, ap.depth + 1
                FROM conversation_messages m
                JOIN active_path ap ON m.id = ap.parent
            )
            SELECT id, conversation_id, parent, role, content, "order",
                   metadata, created_at, updated_at
            FROM active_path
            ORDER BY depth DESC
            "#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(path)
    }

    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError> {
        // COALESCE applies only the fields the caller supplied; None leaves the
        // existing value untouched. In-place edit, no branching.
        let result = sqlx::query(
            "UPDATE conversation_messages
                 SET content = COALESCE($2, content),
                     metadata = COALESCE($3, metadata),
                     updated_at = now()
                 WHERE id = $1",
        )
        .bind(id)
        .bind(edit.content.as_deref())
        .bind(edit.metadata.as_ref())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StorageError::NotFound(format!("Message {id} not found")));
        }

        Ok(())
    }

    async fn delete_message(&self, id: Snowflake) -> Result<bool, StorageError> {
        // Cascades to the subtree via ON DELETE CASCADE on parent. If the
        // active leaf was inside it, fk_conversations_active_leaf sets
        // active_leaf_id to NULL; the caller should then pick a new leaf.
        let result = sqlx::query("DELETE FROM conversation_messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
