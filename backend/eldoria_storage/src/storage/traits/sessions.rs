use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError>;
    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError>;
    async fn delete_session(&self, token: &str) -> Result<bool, StorageError>;
    async fn delete_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError>;
    async fn cleanup_expired(&self) -> Result<u64, StorageError>;
}
