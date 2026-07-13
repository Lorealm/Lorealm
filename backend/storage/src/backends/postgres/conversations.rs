use crate::ConversationStorage;
use crate::error::StorageError;
use crate::models::{
    Conversation, ConversationMessage, ConversationMetadata, MessageEdit, Snowflake,
};
use async_trait::async_trait;

use super::PostgresStorage;

#[async_trait]
impl ConversationStorage for PostgresStorage {
    async fn list_conversations(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
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
                c.realm_id,
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
        sqlx::query(
            "INSERT INTO conversations
                 (id, user_id, character_id, char_version_id, realm_id, active_leaf_id, title)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(conversation.id)
        .bind(conversation.user_id)
        .bind(conversation.character_id)
        .bind(conversation.char_version_id)
        .bind(conversation.realm_id)
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
        let result = sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    async fn add_message(&self, message: &ConversationMessage) -> Result<Snowflake, StorageError> {
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

    async fn get_path_to(
        &self,
        conversation_id: Snowflake,
        message_id: Snowflake,
    ) -> Result<Vec<ConversationMessage>, StorageError> {
        let path = sqlx::query_as::<_, ConversationMessage>(
            r#"
            WITH RECURSIVE message_path AS (
                SELECT m.*, 0 AS depth
                FROM conversation_messages m
                WHERE m.id = $2 AND m.conversation_id = $1

                UNION ALL

                SELECT m.*, mp.depth + 1
                FROM conversation_messages m
                JOIN message_path mp ON m.id = mp.parent
            )
            SELECT id, conversation_id, parent, role, content, "order",
                   metadata, created_at, updated_at
            FROM message_path
            ORDER BY depth DESC
            "#,
        )
        .bind(conversation_id)
        .bind(message_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(path)
    }

    async fn update_message(&self, id: Snowflake, edit: &MessageEdit) -> Result<(), StorageError> {
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
        let result = sqlx::query("DELETE FROM conversation_messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }
}
