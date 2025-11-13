use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use uuid::Uuid;

use crate::ItemSortType;
use crate::storage::CharacterStorage;
use crate::{
    error::StorageError,
    impl_storage_capabilities,
    models::*,
    storage::{AdvancedStorage, ConversationStorage, MessageStorage, StorageCore, UserStorage},
};

#[derive(Debug)]
pub struct PostgresStorage {
    pub pool: PgPool,
}

impl PostgresStorage {
    /// Create a new PostgresStorage with a connection string
    pub async fn new(database_url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await
            .map_err(|e| StorageError::ConnectionError(e.to_string()))?;

        Ok(Self { pool })
    }

    /// Create from an existing pool
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StorageCore for PostgresStorage {
    async fn is_available(&self) -> bool {
        sqlx::query("SELECT 1").fetch_one(&self.pool).await.is_ok()
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        tracing::info!("Running migrations");
        sqlx::migrate!("src/migrations/postgres")
            .run(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        Ok(())
    }

    fn backend_type(&self) -> &str {
        "postgres"
    }
}

#[async_trait]
impl ConversationStorage for PostgresStorage {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        let id = conversation.id;

        // Handle JSONB field - convert Option<String> to Option<serde_json::Value>
        let context_metadata_json: Option<serde_json::Value> = match &conversation.context_metadata
        {
            Some(json_str) => Some(
                serde_json::from_str(json_str)
                    .map_err(|e| StorageError::Database(format!("Invalid JSON: {e}")))?,
            ),
            None => None,
        };

        sqlx::query(
            r#"
            INSERT INTO conversations
                (id, user_id, created_at, updated_at, context_type, context_id,
                 context_version, context_metadata, sync_policy)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            ON CONFLICT (id) DO UPDATE SET
                updated_at = EXCLUDED.updated_at,
                context_type = EXCLUDED.context_type,
                context_id = EXCLUDED.context_id,
                context_version = EXCLUDED.context_version,
                context_metadata = EXCLUDED.context_metadata,
                sync_policy = EXCLUDED.sync_policy
            "#,
        )
        .bind(id)
        .bind(conversation.user_id)
        .bind(conversation.created_at)
        .bind(conversation.updated_at)
        .bind(&conversation.context_type)
        .bind(conversation.context_id)
        .bind(conversation.context_version.map(|v| v as i64))
        .bind(context_metadata_json)
        .bind(&conversation.sync_policy)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        let result = sqlx::query(
            r#"
            SELECT id, user_id, created_at, updated_at, context_type, context_id,
                   context_version, context_metadata, sync_policy
            FROM conversations
            WHERE id = $1
            "#,
        )
        .bind(id)
        .map(|row: PgRow| {
            // Handle JSONB field - convert to JSON string
            let context_metadata_json: Option<serde_json::Value> = row.get("context_metadata");
            let context_metadata =
                context_metadata_json.map(|v| serde_json::to_string(&v).unwrap_or_default());

            Conversation {
                id: row.get("id"),
                user_id: row.get("user_id"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                context_type: row.get("context_type"),
                context_id: row.get("context_id"),
                context_version: row
                    .get::<Option<i64>, _>("context_version")
                    .map(|v| v as u64),
                context_metadata,
                sync_policy: row.get("sync_policy"),
            }
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        let results = sqlx::query(
            r#"
            SELECT cm.id, cm.conversation_id, cm.title, cm.message_count,
                   cm.last_message_at, cm.created_at, cm.updated_at
            FROM conversation_metadata cm
            JOIN conversations c ON c.id = cm.conversation_id
            WHERE c.user_id = $1
            ORDER BY cm.last_message_at DESC
            "#,
        )
        .bind(user_id)
        .map(|row: PgRow| ConversationMetadata {
            id: row.get("id"),
            conversation_id: row.get("conversation_id"),
            title: row.get("title"),
            message_count: row.get::<i32, _>("message_count") as usize,
            last_message_at: row.get("last_message_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM conversations WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl MessageStorage for PostgresStorage {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        let id = message.id;
        let role_str = match message.role {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
        };

        let now = Utc::now();

        // Start a transaction to update both message and metadata
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO messages
                (id, conversation_id, parent_message_id, role, content, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            ON CONFLICT (id) DO UPDATE SET
                content = EXCLUDED.content,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(id)
        .bind(message.conversation_id)
        .bind(message.parent_message_id)
        .bind(role_str)
        .bind(&message.content)
        .bind(message.created_at)
        .bind(message.updated_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        // Update or insert conversation_metadata
        // Generate UUID in Rust instead of using gen_random_uuid()
        let metadata_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO conversation_metadata
                (id, conversation_id, message_count, last_message_at, created_at, updated_at)
            VALUES ($1, $2, 1, $3, $3, $3)
            ON CONFLICT (conversation_id) DO UPDATE SET
                message_count = conversation_metadata.message_count + 1,
                last_message_at = EXCLUDED.last_message_at,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(metadata_id)
        .bind(message.conversation_id)
        .bind(message.created_at)
        .execute(&mut *tx)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        // Update conversation updated_at
        sqlx::query("UPDATE conversations SET updated_at = $1 WHERE id = $2")
            .bind(now)
            .bind(message.conversation_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        let results = sqlx::query(
            r#"
            SELECT id, conversation_id, parent_message_id, role, content, created_at, updated_at
            FROM messages
            WHERE conversation_id = $1
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id)
        .map(|row: PgRow| {
            let role_str: String = row.get("role");
            let role = match role_str.as_str() {
                "user" => MessageRole::User,
                "assistant" => MessageRole::Assistant,
                "system" => MessageRole::System,
                _ => MessageRole::User, // Default fallback
            };

            Message {
                id: row.get("id"),
                conversation_id: row.get("conversation_id"),
                parent_message_id: row.get("parent_message_id"),
                role,
                content: row.get("content"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            }
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }
}

#[async_trait]
impl UserStorage for PostgresStorage {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        let id = user.id;
        let mut query = sqlx::query(
            r#"
            INSERT INTO users (id, permissions, email, display_name, password, created_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (id) DO UPDATE SET
                permissions = EXCLUDED.permissions,
                email = EXCLUDED.email,
                display_name = EXCLUDED.display_name
            "#,
        )
        .bind(id)
        .bind(user.permissions as i64)
        .bind(&user.email)
        .bind(&user.display_name)
        .bind(&user.password)
        .bind(user.created_at);

        // Optionally include password in the update if provided
        if user.password.is_some() {
            query = sqlx::query(
                r#"
                INSERT INTO users (id, permissions, email, display_name, password, created_at)
                VALUES ($1, $2, $3, $4, $5, $6)
                ON CONFLICT (id) DO UPDATE SET
                    permissions = EXCLUDED.permissions,
                    email = EXCLUDED.email,
                    display_name = EXCLUDED.display_name,
                    password = EXCLUDED.password
                "#,
            )
            .bind(id)
            .bind(user.permissions as i64)
            .bind(&user.email)
            .bind(&user.display_name)
            .bind(&user.password)
            .bind(user.created_at);
        }

        query
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        let result = sqlx::query(
            r#"
            SELECT id, permissions, email, display_name, password, created_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .map(|row: PgRow| User {
            id: row.get("id"),
            permissions: row.get::<i64, _>("permissions") as u64,
            email: row.get("email"),
            display_name: row.get("display_name"),
            password: Some(row.get("password")),
            created_at: row.get("created_at"),
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        let result = sqlx::query(
            r#"
            SELECT id, permissions, email, display_name, password, created_at
            FROM users
            WHERE display_name = $1
            "#,
        )
        .bind(name)
        .map(|row: PgRow| User {
            id: row.get("id"),
            permissions: row.get::<i64, _>("permissions") as u64,
            email: row.get("email"),
            display_name: row.get("display_name"),
            password: Some(row.get("password")),
            created_at: row.get("created_at"),
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        let id = session.id;
        let query = sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, token, csrf_token, csrf_rotation_count, csrf_last_rotated_at,
                                ip_address, user_agent, added_at, last_active_at, expires_at, revoked_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(id)
        .bind(session.user_id)
        .bind(&session.token)
        .bind(&session.csrf_token)
        .bind(session.csrf_rotation_count as i32)
        .bind(session.csrf_last_rotated_at)
        .bind(&session.ip_address)
        .bind(&session.user_agent)
        .bind(session.added_at)
        .bind(session.last_active_at)
        .bind(session.expires_at)
        .bind(session.revoked_at);

        query
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        let result = sqlx::query(
            r#"
                SELECT id, user_id, token, csrf_token, csrf_rotation_count, csrf_last_rotated_at,
                       ip_address, user_agent, added_at, last_active_at, expires_at, revoked_at
                FROM sessions
                WHERE id = $1
                "#,
        )
        .bind(id)
        .map(|row: PgRow| Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            token: row.get("token"),
            csrf_token: row.get("csrf_token"),
            csrf_rotation_count: row.get::<i32, _>("csrf_rotation_count") as u32,
            csrf_last_rotated_at: row.get("csrf_last_rotated_at"),
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            added_at: row.get("added_at"),
            last_active_at: row.get("last_active_at"),
            expires_at: row.get("expires_at"),
            revoked_at: row.get("revoked_at"),
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let result = sqlx::query(
            r#"
            SELECT id, user_id, token, csrf_token, csrf_rotation_count, csrf_last_rotated_at,
                   ip_address, user_agent, added_at, last_active_at, expires_at, revoked_at
            FROM sessions
            WHERE token = $1
            "#,
        )
        .bind(token)
        .map(|row: PgRow| Session {
            id: row.get("id"),
            user_id: row.get("user_id"),
            token: row.get("token"),
            csrf_token: row.get("csrf_token"),
            csrf_rotation_count: row.get::<i32, _>("csrf_rotation_count") as u32,
            csrf_last_rotated_at: row.get("csrf_last_rotated_at"),
            ip_address: row.get("ip_address"),
            user_agent: row.get("user_agent"),
            added_at: row.get("added_at"),
            last_active_at: row.get("last_active_at"),
            expires_at: row.get("expires_at"),
            revoked_at: row.get("revoked_at"),
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    // update_session_activity and revoke_session remain the same
    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        if let Some(expires_at) = new_expiration {
            // Update both timestamp and expiration
            sqlx::query(
                r#"
                    UPDATE sessions
                    SET last_active_at = $1, expires_at = $2
                    WHERE id = $3
                    "#,
            )
            .bind(last_active_at)
            .bind(expires_at)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        } else {
            // Update only timestamp
            sqlx::query(
                r#"
                    UPDATE sessions
                    SET last_active_at = $1
                    WHERE id = $2
                    "#,
            )
            .bind(last_active_at)
            .bind(session_id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;
        }

        Ok(())
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        sqlx::query(
            r#"
                UPDATE sessions
                SET revoked_at = $1
                WHERE id = $2
                "#,
        )
        .bind(Utc::now())
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET csrf_token = $1, csrf_rotation_count = $2, csrf_last_rotated_at = $3
            WHERE id = $4
            "#,
        )
        .bind(&new_csrf_token)
        .bind(rotation_count as i32)
        .bind(rotated_at)
        .bind(session_id)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn invalidate_sessions(
        &self,
        user_id: Uuid,
        ip: &str,
        user_agent: &str,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET revoked_at = NOW()
            WHERE user_id = $1
              AND ip_address = $2
              AND user_agent = $3
              AND expires_at > NOW()
              AND revoked_at IS NULL
            "#,
        )
        .bind(user_id)
        .bind(ip)
        .bind(user_agent)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl CharacterStorage for PostgresStorage {
    async fn get_character(&self, id: Uuid) -> Result<Option<Character>, StorageError> {
        let result = sqlx::query(
            r#"
            SELECT id, user_id, spec, version, name, display_name, personality,
                   creator, creator_notes, description, created_at, updated_at
            FROM characters
            WHERE id = $1
            "#,
        )
        .bind(id)
        .map(|row: PgRow| Character {
            id: row.get("id"),
            user_id: row.get("user_id"),
            spec: row.get("spec"),
            version: row.get("version"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            personality: row.get("personality"),
            creator: row.get("creator"),
            creator_notes: row.get("creator_notes"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(result)
    }

    async fn get_character_tags(&self, id: Uuid) -> Result<Vec<CharacterTag>, StorageError> {
        let results = sqlx::query(
            r#"
            SELECT character_id, tag
            FROM character_tags
            WHERE character_id = $1
            ORDER BY tag
            "#,
        )
        .bind(id)
        .map(|row: PgRow| CharacterTag {
            character_id: row.get("character_id"),
            tag: row.get("tag"),
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }

    async fn get_character_greetings(
        &self,
        id: Uuid,
    ) -> Result<Vec<CharacterGreeting>, StorageError> {
        let results = sqlx::query(
            r#"
            SELECT id, character_id, greeting_text, display_order
            FROM character_greetings
            WHERE character_id = $1
            ORDER BY display_order
            "#,
        )
        .bind(id)
        .map(|row: PgRow| CharacterGreeting {
            id: row.get("id"),
            character_id: row.get("character_id"),
            greeting: row.get("greeting_text"),
            display_order: row.get::<i32, _>("display_order") as usize,
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }

    async fn get_characters_with_tags(
        &self,
        tags: Vec<String>,
    ) -> Result<Vec<Character>, StorageError> {
        if tags.is_empty() {
            return Ok(Vec::new());
        }

        // Find characters that have ALL the specified tags
        let tag_count = tags.len() as i64;

        let results = sqlx::query(
            r#"
            SELECT c.id, c.user_id, c.spec, c.version, c.name, c.display_name,
                   c.personality, c.creator, c.creator_notes, c.description,
                   c.created_at, c.updated_at
            FROM characters c
            INNER JOIN character_tags ct ON c.id = ct.character_id
            WHERE ct.tag = ANY($1)
            GROUP BY c.id, c.user_id, c.spec, c.version, c.name, c.display_name,
                     c.personality, c.creator, c.creator_notes, c.description,
                     c.created_at, c.updated_at
            HAVING COUNT(DISTINCT ct.tag) = $2
            ORDER BY c.name
            "#,
        )
        .bind(&tags)
        .bind(tag_count)
        .map(|row: PgRow| Character {
            id: row.get("id"),
            user_id: row.get("user_id"),
            spec: row.get("spec"),
            version: row.get("version"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            personality: row.get("personality"),
            creator: row.get("creator"),
            creator_notes: row.get("creator_notes"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }

    async fn save_character(&self, character: &Character) -> Result<Uuid, StorageError> {
        let id = character.id;

        sqlx::query(
            r#"
            INSERT INTO characters
                (id, user_id, spec, version, name, display_name, personality,
                 creator, creator_notes, description, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                user_id = EXCLUDED.user_id,
                spec = EXCLUDED.spec,
                version = EXCLUDED.version,
                name = EXCLUDED.name,
                display_name = EXCLUDED.display_name,
                personality = EXCLUDED.personality,
                creator = EXCLUDED.creator,
                creator_notes = EXCLUDED.creator_notes,
                description = EXCLUDED.description,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(id)
        .bind(character.user_id)
        .bind(&character.spec)
        .bind(character.version)
        .bind(&character.name)
        .bind(&character.display_name)
        .bind(&character.personality)
        .bind(&character.creator)
        .bind(&character.creator_notes)
        .bind(&character.description)
        .bind(character.created_at)
        .bind(character.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(id)
    }

    async fn save_character_tag(&self, tag: &CharacterTag) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO character_tags (character_id, tag)
            VALUES ($1, $2)
            ON CONFLICT (character_id, tag) DO NOTHING
            "#,
        )
        .bind(tag.character_id)
        .bind(&tag.tag)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn save_character_greeting(
        &self,
        greeting: &CharacterGreeting,
    ) -> Result<(), StorageError> {
        sqlx::query(
            r#"
            INSERT INTO character_greetings (id, character_id, greeting_text, display_order)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
                greeting_text = EXCLUDED.greeting_text,
                display_order = EXCLUDED.display_order
            "#,
        )
        .bind(greeting.id)
        .bind(greeting.character_id)
        .bind(&greeting.greeting)
        .bind(greeting.display_order as i32)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn delete_character(&self, id: Uuid) -> Result<(), StorageError> {
        // CASCADE will handle deletion of tags, greetings, and example messages
        sqlx::query("DELETE FROM characters WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(())
    }

    async fn list_characters(&self, user_id: Uuid) -> Result<Vec<Character>, StorageError> {
        let results = sqlx::query(
            r#"
            SELECT id, user_id, spec, version, name, display_name, personality,
                   creator, creator_notes, description, created_at, updated_at
            FROM characters
            WHERE user_id = $1
            ORDER BY name
            "#,
        )
        .bind(user_id)
        .map(|row: PgRow| Character {
            id: row.get("id"),
            user_id: row.get("user_id"),
            spec: row.get("spec"),
            version: row.get("version"),
            name: row.get("name"),
            display_name: row.get("display_name"),
            personality: row.get("personality"),
            creator: row.get("creator"),
            creator_notes: row.get("creator_notes"),
            description: row.get("description"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }
}

#[async_trait]
impl AdvancedStorage for PostgresStorage {
    async fn search_conversations(
        &self,
        query: &str,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        // Note: Your migration doesn't include the full-text search index
        // You may want to add this to your migration:
        // CREATE INDEX idx_messages_content_fts ON messages USING gin(to_tsvector('english', content));

        let results = sqlx::query(
            r#"
            SELECT DISTINCT cm.id, cm.conversation_id, cm.title, cm.message_count,
                   cm.last_message_at, cm.created_at, cm.updated_at
            FROM conversation_metadata cm
            JOIN conversations c ON c.id = cm.conversation_id
            JOIN messages m ON m.conversation_id = c.id
            WHERE c.user_id = $1
              AND to_tsvector('english', m.content) @@ plainto_tsquery('english', $2)
            ORDER BY cm.last_message_at DESC
            "#,
        )
        .bind(user_id)
        .bind(query)
        .map(|row: PgRow| ConversationMetadata {
            id: row.get("id"),
            conversation_id: row.get("conversation_id"),
            title: row.get("title"),
            message_count: row.get::<i32, _>("message_count") as usize,
            last_message_at: row.get("last_message_at"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }

    async fn get_conversations_paginated(
        &self,
        sort_type: ItemSortType,
        user_id: Uuid,
        page_size: usize,
        page: usize,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        let offset = page * page_size;
        let limit = page_size as i64;

        let order_clause = match sort_type {
            ItemSortType::Id => "cm.id",
            ItemSortType::LatestUpdate => "cm.updated_at DESC",
            ItemSortType::LatestAdded => "cm.created_at DESC",
            ItemSortType::Random(seed) => {
                // For reproducible randomization using PostgreSQL's hash functions
                let query_str = format!(
                    r#"
                    SELECT cm.id, cm.conversation_id, cm.title, cm.message_count,
                           cm.last_message_at, cm.created_at, cm.updated_at
                    FROM conversation_metadata cm
                    JOIN conversations c ON c.id = cm.conversation_id
                    WHERE c.user_id = $1
                    ORDER BY hashtext(cm.id::text || '{seed}')
                    LIMIT $2 OFFSET $3
                    "#,
                );

                let results = sqlx::query(&query_str)
                    .bind(user_id)
                    .bind(limit)
                    .bind(offset as i64)
                    .map(|row: PgRow| ConversationMetadata {
                        id: row.get("id"),
                        conversation_id: row.get("conversation_id"),
                        title: row.get("title"),
                        message_count: row.get::<i32, _>("message_count") as usize,
                        last_message_at: row.get("last_message_at"),
                        created_at: row.get("created_at"),
                        updated_at: row.get("updated_at"),
                    })
                    .fetch_all(&self.pool)
                    .await
                    .map_err(|e| StorageError::Database(e.to_string()))?;

                return Ok(results);
            }
            ItemSortType::Tags => {
                return Err(StorageError::NotImplemented(
                    "Tags sorting not yet implemented".to_string(),
                ));
            }
        };

        let query_str = format!(
            r#"
            SELECT cm.id, cm.conversation_id, cm.title, cm.message_count,
                   cm.last_message_at, cm.created_at, cm.updated_at
            FROM conversation_metadata cm
            JOIN conversations c ON c.id = cm.conversation_id
            WHERE c.user_id = $1
            ORDER BY {order_clause}
            LIMIT $2 OFFSET $3
            "#
        );

        let results = sqlx::query(&query_str)
            .bind(user_id)
            .bind(limit)
            .bind(offset as i64)
            .map(|row: PgRow| ConversationMetadata {
                id: row.get("id"),
                conversation_id: row.get("conversation_id"),
                title: row.get("title"),
                message_count: row.get::<i32, _>("message_count") as usize,
                last_message_at: row.get("last_message_at"),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
            })
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(results)
    }
}

impl_storage_capabilities!(PostgresStorage, {
    supports_conversations: true,
    supports_messages: true,
    supports_users: true,
    supports_sessions: true,
    supports_characters: true
});
