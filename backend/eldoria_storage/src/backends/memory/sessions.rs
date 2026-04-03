use crate::SessionStorage;
use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;
use chrono::Utc;

use super::MemoryStorage;

#[async_trait]
impl SessionStorage for MemoryStorage {
    async fn create_session(&self, input: &CreateSession) -> Result<Session, StorageError> {
        let mut sessions = self.sessions.write().await;

        let now = Utc::now();
        let session = Session {
            id: input.id,
            user_id: input.user_id,
            token: input.token.clone(),
            ip_address: input.ip_address.clone(),
            user_agent: input.user_agent.clone(),
            created_at: now,
            expires_at: input.expires_at,
        };

        sessions.insert(session.token.clone(), session.clone());
        Ok(session)
    }

    async fn get_session_by_token(&self, token: &str) -> Result<Option<Session>, StorageError> {
        let sessions = self.sessions.read().await;
        let now = Utc::now();

        Ok(sessions.get(token).filter(|s| s.expires_at > now).cloned())
    }

    async fn delete_session(&self, token: &str) -> Result<bool, StorageError> {
        Ok(self.sessions.write().await.remove(token).is_some())
    }

    async fn delete_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        let mut sessions = self.sessions.write().await;
        let before = sessions.len();
        sessions.retain(|_, s| s.user_id != user_id);
        Ok((before - sessions.len()) as u64)
    }

    async fn cleanup_expired(&self) -> Result<u64, StorageError> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let before = sessions.len();
        sessions.retain(|_, s| s.expires_at > now);
        Ok((before - sessions.len()) as u64)
    }
}
