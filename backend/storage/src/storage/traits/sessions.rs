use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError>;
    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError>;
    async fn delete_session(&self, token: &str) -> Result<bool, StorageError>;
    async fn delete_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError>;
    async fn list_user_sessions(&self, user_id: Snowflake) -> Result<Vec<Session>, StorageError>;
    async fn cleanup_expired(&self) -> Result<u64, StorageError>;
    async fn rotate_session_token(
        &self,
        session_id: Snowflake,
        new_token_hash: &str,
        old_token_hash: &str,
        grace_expires_at: DateTime<Utc>,
        new_expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError>;
    async fn extend_session(
        &self,
        session_id: Snowflake,
        new_expires_at: DateTime<Utc>,
    ) -> Result<(), StorageError>;
    async fn revoke_session(&self, id: Snowflake) -> Result<(), StorageError>;
    async fn revoke_all_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError>;
    async fn rotate_csrf_token(
        &self,
        session_id: Snowflake,
        new_csrf_hash: &str,
    ) -> Result<(), StorageError>;
    async fn increment_request_count(&self, session_id: Snowflake) -> Result<i32, StorageError>;
}
