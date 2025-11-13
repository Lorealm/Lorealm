use std::sync::Arc;

use chrono::{DateTime, Utc};
use storage_system::{Storage, StorageError};
use uuid::Uuid;

use crate::{
    models::StorageService,
    services::authentication::{Session, SessionConfig},
};

pub struct SessionService<S: Storage + ?Sized> {
    storage: Arc<S>,
    config: SessionConfig,
}

impl<S: Storage + ?Sized> StorageService<S> for SessionService<S> {
    fn new(storage: Arc<S>) -> Self {
        Self {
            storage,
            config: SessionConfig::default(),
        }
    }
}

impl<S: Storage + ?Sized> SessionService<S> {
    // Add config to the service
    pub fn new_with_config(storage: Arc<S>, config: SessionConfig) -> Self {
        Self { storage, config }
    }

    /// Retrieves a session object from the storage backend. Does not perform any session validation.
    pub async fn get_session(&self, token: &str) -> Result<Session, StorageError> {
        if let Some(storage_session) = self.storage.get_session_by_token(token).await? {
            let mut session: Session = storage_session.into();
            // Apply service config to retrieved session
            session.config = self.config.clone();
            return Ok(session);
        }

        Err(StorageError::NotFound(format!(
            "SessionService: Session '{token}' not found"
        )))
    }

    pub async fn save_session(&self, session: Session) -> Result<(), StorageError> {
        let _ = self.storage.save_session(&session.into()).await?;
        Ok(())
    }

    pub async fn update_session_activity(
        &self,
        session_id: Uuid,
        last_active_at: DateTime<Utc>,
        new_expiration: Option<DateTime<Utc>>,
    ) -> Result<(), StorageError> {
        self.storage
            .update_session_activity(session_id, last_active_at, new_expiration)
            .await
    }

    pub async fn revoke_session(&self, session_id: Uuid) -> Result<(), StorageError> {
        let _ = self.storage.revoke_session(session_id).await?;
        Ok(())
    }

    pub async fn update_csrf_token(
        &self,
        session_id: Uuid,
        new_csrf_token: String,
        rotation_count: u32,
        rotated_at: DateTime<Utc>,
    ) -> Result<(), StorageError> {
        self.storage
            .update_csrf_token(session_id, new_csrf_token, rotation_count, rotated_at)
            .await
    }
}
