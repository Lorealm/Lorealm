use crate::SessionStorage;
use crate::error::StorageError;
use crate::models::{CreateSession, Session, Snowflake};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

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
            csrf_token: input.csrf.clone(),
            created_at: now,
            expires_at: input.expires_at,
            revoked_at: None,
            last_rotated_at: Utc::now(),
            previous_token: None,
            grace_expires_at: None,
            last_csrf_rotated_at: Utc::now(),
            requests_since_csrf_rotation: 0,
            ip_address: input.ip_address.clone(),
            user_agent: input.user_agent.clone(),
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

    async fn rotate_session_token(
        &self,
        session_id: Snowflake,
        new_token_hash: &str,
        old_token_hash: &str,
        grace_expires_at: DateTime<Utc>,
        new_expires_at: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .values()
            .find(|s| s.id == session_id)
            .cloned()
            .ok_or(StorageError::NotFound(session_id.to_string()))?;

        sessions.remove(&session.token);

        let mut updated = session;
        updated.previous_token = Some(old_token_hash.to_string());
        updated.token = new_token_hash.to_string();
        updated.grace_expires_at = Some(grace_expires_at);
        updated.last_rotated_at = Utc::now();
        if let Some(expires) = new_expires_at {
            updated.expires_at = expires;
        }

        sessions.insert(updated.token.clone(), updated);
        Ok(())
    }

    async fn extend_session(
        &self,
        session_id: Snowflake,
        new_expires_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .values_mut()
            .find(|s| s.id == session_id)
            .ok_or(StorageError::NotFound(session_id.to_string()))?;

        session.expires_at = new_expires_at;
        Ok(())
    }

    async fn revoke_session(&self, id: Snowflake) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .values_mut()
            .find(|s| s.id == id)
            .ok_or(StorageError::NotFound(id.to_string()))?;

        session.revoked_at = Some(Utc::now());
        Ok(())
    }

    async fn revoke_all_user_sessions(&self, user_id: Snowflake) -> Result<u64, StorageError> {
        let mut sessions = self.sessions.write().await;
        let now = Utc::now();
        let mut count = 0u64;

        for session in sessions.values_mut() {
            if session.user_id == user_id && session.revoked_at.is_none() {
                session.revoked_at = Some(now);
                count += 1;
            }
        }

        Ok(count)
    }

    async fn rotate_csrf_token(
        &self,
        session_id: Snowflake,
        new_csrf_hash: &str,
    ) -> Result<(), StorageError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .values_mut()
            .find(|s| s.id == session_id)
            .ok_or(StorageError::NotFound(session_id.to_string()))?;

        session.csrf_token = new_csrf_hash.to_string();
        session.last_csrf_rotated_at = Utc::now();
        session.requests_since_csrf_rotation = 0;
        Ok(())
    }

    async fn increment_request_count(&self, session_id: Snowflake) -> Result<i32, StorageError> {
        let mut sessions = self.sessions.write().await;

        let session = sessions
            .values_mut()
            .find(|s| s.id == session_id)
            .ok_or(StorageError::NotFound(session_id.to_string()))?;

        session.requests_since_csrf_rotation += 1;
        Ok(session.requests_since_csrf_rotation)
    }
}
