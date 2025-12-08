use crate::StorageError;
use crate::models::*;
use crate::storage::{
    CharacterStorage, ConversationStorage, MessageStorage, Storage, StorageCapabilities,
    StorageCore, UserStorage, TemplateStorage
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use redis::{AsyncCommands, Client};
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;
use uuid::Uuid;

#[cfg(feature = "redis")]
#[derive(Debug)]
pub struct CachedStorage {
    /// Underlying storage backend
    backend: Arc<dyn Storage>,
    /// Redis client
    redis: Client,
    /// Cache TTL in seconds
    ttl: u64,
    /// Key prefix for namespacing
    prefix: String,
}

#[cfg(feature = "redis")]
impl CachedStorage {
    pub fn new(
        backend: Arc<dyn Storage>,
        redis_url: &str,
        ttl: u64,
        prefix: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let redis = Client::open(redis_url)
            .map_err(|e| StorageError::Configuration(format!("Redis connection failed: {e}")))?;

        Ok(Self {
            backend,
            redis,
            ttl,
            prefix: prefix.into(),
        })
    }

    /// Generate cache key
    fn cache_key(&self, entity: &str, id: &str) -> String {
        format!("{}:{}:{}", self.prefix, entity, id)
    }

    /// Generate pattern for invalidation
    fn cache_pattern(&self, entity: &str, pattern: &str) -> String {
        format!("{}:{}:{}", self.prefix, entity, pattern)
    }

    /// Get from cache or fetch from backend
    async fn get_with_cache<T, F>(&self, key: &str, fetcher: F) -> Result<Option<T>, StorageError>
    where
        T: Serialize + DeserializeOwned,
        F: std::future::Future<Output = Result<Option<T>, StorageError>>,
    {
        // Try cache first
        if let Ok(Some(cached)) = self.get_cached(key).await {
            return Ok(Some(cached));
        }

        // Cache miss - fetch from backend
        let result = fetcher.await?;

        // Store in cache if found
        if let Some(ref value) = result {
            if let Ok(serialized) = serde_json::to_vec(value) {
                let mut conn = self
                    .redis
                    .get_multiplexed_async_connection()
                    .await
                    .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
                let _: Result<(), _> = conn.set_ex(key, serialized, self.ttl).await;
            }
        }

        Ok(result)
    }

    async fn get_cached<T>(&self, key: &str) -> Result<Option<T>, StorageError>
    where
        T: Serialize + DeserializeOwned,
    {
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Set in cache and backend
    async fn set_cached<T>(&self, key: &str, value: &T) -> Result<(), StorageError>
    where
        T: Serialize,
    {
        // Store in cache
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(serialized) = serde_json::to_vec(value) {
            let _: Result<(), _> = conn.set_ex(key, serialized, self.ttl).await;
        }

        Ok(())
    }

    /// Invalidate cache entries
    async fn invalidate(&self, pattern: &str) -> Result<(), StorageError> {
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        // Find all matching keys
        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        // Delete them
        if !keys.is_empty() {
            let _: Result<(), _> = conn.del(&keys).await;
        }

        Ok(())
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl StorageCore for CachedStorage {
    async fn is_available(&self) -> bool {
        // Check both backend and Redis
        self.backend.is_available().await
            && self.redis.get_multiplexed_async_connection().await.is_ok()
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        // Cannot initialize through cache wrapper - backend should be initialized separately
        Err(StorageError::InvalidOperation(
            "Cannot initialize cached storage - initialize backend separately".to_string(),
        ))
    }

    fn backend_type(&self) -> &str {
        "cached"
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl ConversationStorage for CachedStorage {
    async fn save_conversation(&self, conversation: &Conversation) -> Result<Uuid, StorageError> {
        // Save to backend first
        let result = self.backend.save_conversation(conversation).await?;

        // Update cache
        let key = self.cache_key("conversation", &conversation.id.to_string());
        self.set_cached(&key, conversation).await?;

        // Invalidate list cache for this user
        let list_pattern =
            self.cache_pattern("conversation_list", &format!("{}:*", conversation.user_id));
        self.invalidate(&list_pattern).await?;

        Ok(result)
    }

    async fn get_conversation(&self, id: Uuid) -> Result<Option<Conversation>, StorageError> {
        let key = self.cache_key("conversation", &id.to_string());
        self.get_with_cache(&key, self.backend.get_conversation(id))
            .await
    }

    async fn list_conversations(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<ConversationMetadata>, StorageError> {
        let key = self.cache_key("conversation_list", &user_id.to_string());

        // For lists, we might want shorter TTL
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(value);
            }
        }

        // Fetch from backend
        let result = self.backend.list_conversations(user_id).await?;

        // Cache with shorter TTL for lists (they change more often)
        if let Ok(serialized) = serde_json::to_vec(&result) {
            let _: Result<(), _> = conn.set_ex(&key, serialized, self.ttl / 2).await;
        }

        Ok(result)
    }

    async fn delete_conversation(&self, id: Uuid) -> Result<(), StorageError> {
        // Delete from backend
        self.backend.delete_conversation(id).await?;

        // Invalidate cache
        let key = self.cache_key("conversation", &id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&key).await;

        // Also invalidate messages cache
        let messages_pattern = self.cache_pattern("messages", &format!("{id}:*"));
        self.invalidate(&messages_pattern).await?;

        Ok(())
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl MessageStorage for CachedStorage {
    async fn save_message(&self, message: &Message) -> Result<Uuid, StorageError> {
        let result = self.backend.save_message(message).await?;

        // Invalidate messages cache for this conversation
        let key = self.cache_key("messages", &message.conversation_id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&key).await;

        Ok(result)
    }

    async fn get_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, StorageError> {
        let key = self.cache_key("messages", &conversation_id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(value);
            }
        }

        // Fetch from backend
        let result = self.backend.get_messages(conversation_id).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&result) {
            let _: Result<(), _> = conn.set_ex(&key, serialized, self.ttl).await;
        }

        Ok(result)
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl UserStorage for CachedStorage {
    async fn save_user(&self, user: &User) -> Result<Uuid, StorageError> {
        let result = self.backend.save_user(user).await?;

        let key = self.cache_key("user", &user.id.to_string());
        self.set_cached(&key, user).await?;

        Ok(result)
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>, StorageError> {
        let key = self.cache_key("user", &id.to_string());
        self.get_with_cache(&key, self.backend.get_user_by_id(id))
            .await
    }

    async fn get_user_by_name(&self, name: &str) -> Result<Option<User>, StorageError> {
        // Generally used for logins, which we do not want to cache
        self.backend.get_user_by_name(name).await
    }

    async fn save_session(&self, session: &Session) -> Result<Uuid, StorageError> {
        let result = self.backend.save_session(session).await?;

        let key = self.cache_key("user:session", &session.id.to_string());
        self.set_cached(&key, session).await?;

        Ok(result)
    }

    async fn get_session_by_id(&self, id: Uuid) -> Result<Option<Session>, StorageError> {
        let key = self.cache_key("user:session", &id.to_string());
        self.get_with_cache(&key, self.backend.get_session_by_id(id))
            .await
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let key = self.cache_key("user:session:token", token);
        self.get_with_cache(&key, self.backend.get_session_by_token(token))
            .await
    }

    async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        self.backend
            .update_session_activity(session_id, last_active_at, new_expiration)
            .await?;

        let key = self.cache_key("user:session", &session_id.to_string());
        if let Ok(Some(mut session)) = self.get_cached::<Session>(&key).await {
            session.last_active_at = last_active_at;
            if let Some(expires_at) = new_expiration {
                session.expires_at = expires_at;
            }
            self.set_cached(&key, &session).await?;
        }

        Ok(())
    }

    async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        self.backend.revoke_session(session_id).await?;

        let key = self.cache_key("user:session", &session_id.to_string());
        if let Ok(Some(mut session)) = self.get_cached::<Session>(&key).await {
            session.revoked_at = Some(Utc::now());
            self.set_cached(&key, &session).await?;
        }

        Ok(())
    }

    async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.backend
            .update_csrf_token(
                session_id,
                new_csrf_token.clone(),
                rotation_count,
                rotated_at,
            )
            .await?;

        let key = self.cache_key("user:session", &session_id.to_string());
        if let Ok(Some(mut session)) = self.get_cached::<Session>(&key).await {
            session.csrf_token = new_csrf_token;
            session.csrf_rotation_count = rotation_count;
            session.csrf_last_rotated_at = rotated_at;
            self.set_cached(&key, &session).await?;
        }

        Ok(())
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl CharacterStorage for CachedStorage {
    async fn get_character(&self, id: Uuid) -> Result<Option<Character>, StorageError> {
        let key = self.cache_key("character", &id.to_string());
        self.get_with_cache(&key, self.backend.get_character(id))
            .await
    }

    async fn get_character_tags(&self, id: Uuid) -> Result<Vec<CharacterTag>, StorageError> {
        let key = self.cache_key("character_tags", &id.to_string());

        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(value);
            }
        }

        // Fetch from backend
        let result = self.backend.get_character_tags(id).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&result) {
            let _: Result<(), _> = conn.set_ex(&key, serialized, self.ttl).await;
        }

        Ok(result)
    }

    async fn get_character_greetings(
        &self,
        id: Uuid,
    ) -> Result<Vec<CharacterGreeting>, StorageError> {
        let key = self.cache_key("character_greetings", &id.to_string());

        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(value);
            }
        }

        // Fetch from backend
        let result = self.backend.get_character_greetings(id).await?;

        // Cache the result
        if let Ok(serialized) = serde_json::to_vec(&result) {
            let _: Result<(), _> = conn.set_ex(&key, serialized, self.ttl).await;
        }

        Ok(result)
    }

    async fn get_characters_with_tags(
        &self,
        tags: Vec<String>,
    ) -> Result<Vec<Character>, StorageError> {
        // Don't cache complex queries, just pass through
        self.backend.get_characters_with_tags(tags).await
    }

    async fn save_character(&self, character: &Character) -> Result<Uuid, StorageError> {
        let result = self.backend.save_character(character).await?;

        // Update cache
        let key = self.cache_key("character", &character.id.to_string());
        self.set_cached(&key, character).await?;

        Ok(result)
    }

    async fn save_character_tag(&self, tag: &CharacterTag) -> Result<(), StorageError> {
        self.backend.save_character_tag(tag).await?;

        // Invalidate tags cache for this character
        let key = self.cache_key("character_tags", &tag.character_id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&key).await;

        Ok(())
    }

    async fn save_character_greeting(
        &self,
        greeting: &CharacterGreeting,
    ) -> Result<(), StorageError> {
        self.backend.save_character_greeting(greeting).await?;

        // Invalidate greetings cache for this character
        let key = self.cache_key("character_greetings", &greeting.character_id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&key).await;

        Ok(())
    }

    async fn delete_character(&self, id: Uuid) -> Result<(), StorageError> {
        self.backend.delete_character(id).await?;

        // Invalidate all character caches
        let char_key = self.cache_key("character", &id.to_string());
        let tags_key = self.cache_key("character_tags", &id.to_string());
        let greetings_key = self.cache_key("character_greetings", &id.to_string());

        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&[char_key, tags_key, greetings_key]).await;

        Ok(())
    }

    async fn list_characters(&self, user_id: Uuid) -> Result<Vec<Character>, StorageError> {
        // Don't cache list operations, just pass through
        self.backend.list_characters(user_id).await
    }
}

#[async_trait]
#[cfg(feature = "redis")]
impl TemplateStorage for CachedStorage {
    async fn save_template(&self, template: &Template) -> Result<Uuid, StorageError> {
        // Save to backend first
        let result = self.backend.save_template(template).await?;

        // Update cache
        let key = self.cache_key("template", &template.id.to_string());
        self.set_cached(&key, template).await?;

        // Invalidate list cache for this user
        let list_pattern =
            self.cache_pattern("template_list", &format!("{}:*", template.owner_id));
        self.invalidate(&list_pattern).await?;

        Ok(result)
    }

    async fn get_template(&self, id: Uuid) -> Result<Option<Template>, StorageError> {
        let key = self.cache_key("template", &id.to_string());
        self.get_with_cache(&key, self.backend.get_template(id))
            .await
    }

    async fn get_template_owned(
        &self,
        id: Uuid,
        owner_id: Uuid,
    ) -> Result<Option<Template>, StorageError> {
        // For owned templates, we need to check ownership
        // We can still use cache but verify ownership after retrieval
        let key = self.cache_key("template", &id.to_string());

        match self.get_with_cache(&key, self.backend.get_template(id)).await? {
            Some(template) if template.owner_id == owner_id => Ok(Some(template)),
            Some(_) => Err(StorageError::PermissionDenied(
                "Template not owned by user".to_string(),
            )),
            None => Ok(None),
        }
    }

    async fn list_templates(&self, owner_id: Uuid) -> Result<Vec<TemplateMetadata>, StorageError> {
        let key = self.cache_key("template_list", &owner_id.to_string());

        // For lists, we might want shorter TTL
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;

        if let Ok(Some(cached)) = conn.get::<_, Option<Vec<u8>>>(&key).await {
            if let Ok(value) = serde_json::from_slice(&cached) {
                return Ok(value);
            }
        }

        // Fetch from backend
        let result = self.backend.list_templates(owner_id).await?;

        // Cache with shorter TTL for lists (they change more often)
        if let Ok(serialized) = serde_json::to_vec(&result) {
            let _: Result<(), _> = conn.set_ex(&key, serialized, self.ttl / 2).await;
        }

        Ok(result)
    }

    async fn delete_template(&self, id: Uuid, owner_id: Uuid) -> Result<(), StorageError> {
        // Delete from backend
        self.backend.delete_template(id, owner_id).await?;

        // Invalidate cache
        let key = self.cache_key("template", &id.to_string());
        let mut conn = self
            .redis
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| StorageError::Database(format!("Redis error: {e}")))?;
        let _: Result<(), _> = conn.del(&key).await;

        // Invalidate list cache for this user
        let list_pattern = self.cache_pattern("template_list", &format!("{owner_id}:*"));
        self.invalidate(&list_pattern).await?;

        Ok(())
    }

    async fn template_exists(&self, id: Uuid) -> Result<bool, StorageError> {
        // Check cache first
        let key = self.cache_key("template", &id.to_string());

        if let Ok(Some(_)) = self.get_cached::<Template>(&key).await {
            return Ok(true);
        }

        // Fall back to backend
        self.backend.template_exists(id).await
    }

    async fn template_name_exists(
        &self,
        owner_id: Uuid,
        name: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, StorageError> {
        // This is a query operation - don't cache, just pass through
        self.backend
            .template_name_exists(owner_id, name, exclude_id)
            .await
    }
}

#[cfg(feature = "redis")]
impl StorageCapabilities for CachedStorage {
    // Cache supports whatever the underlying backend supports
    // we'll report all as true and let the backend return NotImplemented if needed
    fn supports_conversations(&self) -> bool {
        true
    }
    fn supports_messages(&self) -> bool {
        true
    }
    fn supports_users(&self) -> bool {
        true
    }
    fn supports_sessions(&self) -> bool {
        true
    }
    fn supports_characters(&self) -> bool {
        true
    }
    fn supports_templates(&self) -> bool {
        true
    }
}
