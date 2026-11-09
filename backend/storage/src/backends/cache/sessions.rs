use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use crate::storage::SessionStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

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
        // so we just let them expire naturally. Backend is the source of truth.
        self.backend.delete_user_sessions(user_id).await
    }

    async fn list_user_sessions(&self, user_id: Snowflake) -> Result<Vec<Session>, StorageError> {
        // Keyed by token, so a per-user listing can only come from the backend.
        self.backend.list_user_sessions(user_id).await
    }

    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        self.backend.cleanup_expired().await
    }

    async fn rotate_session_token(
        &self,
        session_id: Snowflake,
        new_token_hash: &str,
        old_token_hash: &str,
        grace_expires_at: DateTime<Utc>,
        new_expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        self.backend
            .rotate_session_token(
                session_id,
                new_token_hash,
                old_token_hash,
                grace_expires_at,
                new_expires_at,
            )
            .await?;

        self.cache_del(&self.key("session", old_token_hash)).await;
        Ok(())
    }

    async fn extend_session(
        &self,
        session_id: Snowflake,
        new_expires_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.backend
            .extend_session(session_id, new_expires_at)
            .await?;
        Ok(())
    }

    async fn revoke_session(&self, id: Snowflake) -> Result<(), StorageError> {
        self.backend.revoke_session(id).await?;
        Ok(())
    }

    async fn revoke_all_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        // Cached sessions would otherwise stay valid until they expire.
        let sessions = self.backend.list_user_sessions(user_id).await?;
        let revoked = self.backend.revoke_all_user_sessions(user_id).await?;
        for session in sessions {
            self.cache_del(&self.key("session", &session.token)).await;
            if let Some(previous) = &session.previous_token {
                self.cache_del(&self.key("session", previous)).await;
            }
        }
        Ok(revoked)
    }

    async fn rotate_csrf_token(
        &self,
        session_id: Snowflake,
        new_csrf_hash: &str,
    ) -> Result<(), StorageError> {
        self.backend
            .rotate_csrf_token(session_id, new_csrf_hash)
            .await?;
        Ok(())
    }

    async fn increment_request_count(&self, session_id: Snowflake) -> Result<i32, StorageError> {
        self.backend.increment_request_count(session_id).await
    }
}
