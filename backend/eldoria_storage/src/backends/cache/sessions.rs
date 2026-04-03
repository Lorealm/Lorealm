use crate::SessionStorage;
use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;

use super::CachedStorage;

#[async_trait]
impl SessionStorage for CachedStorage {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError> {
        let session = self.backend.create_session(input).await?;
        self.cache_set(&self.key("session", &session.token), &session)
            .await;
        Ok(session)
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let key = self.key("session", token);
        self.get_through(&key, self.backend.get_session_by_token(token))
            .await
    }

    async fn delete_session(&self, token: &str) -> Result<bool, StorageError> {
        let result = self.backend.delete_session(token).await?;
        if result {
            self.cache_del(&self.key("session", token)).await;
        }
        Ok(result)
    }

    async fn delete_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        // Can't efficiently invalidate all session keys for a user without scanning,
        // so just let them expire naturally. Backend is the source of truth.
        self.backend.delete_user_sessions(user_id).await
    }

    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        self.backend.cleanup_expired().await
    }
}
