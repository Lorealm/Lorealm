use std::{sync::Arc, time::Duration};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use eldoria_storage::{
    Storage,
    models::{CreateSession, Session, Snowflake},
};
use rand::{TryRng, rngs::SysRng};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    config::{CsrfRotationPolicy, SessionConfig, SlidingPolicy},
    errors::AuthError,
    extract::UserDevice,
    services::{
        session::{SessionCreated, ValidatedSession},
        snowflake::SnowflakeService,
    },
};

#[derive(Clone)]
pub struct SessionService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    config: SessionConfig,
}

impl SessionService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        config: SessionConfig,
    ) -> Self {
        Self {
            storage,
            snowflake,
            config,
        }
    }

    pub async fn create_session(
        &self,
        user_id: Snowflake,
        server_secret: &SecretString,
        user_device: UserDevice,
    ) -> Result<SessionCreated, AuthError> {
        let session_token_raw = generate_token(self.config.token_length)?;
        let csrf_token_raw = generate_token(self.config.csrf_token_length)?;
        let now = Utc::now();
        let expires_at = now + Duration::from_secs(self.config.ttl_seconds as u64);

        let secret = server_secret.expose_secret().as_bytes();

        let session_token = eldoria_encryption::hash_session_token(&session_token_raw, secret)?;
        let csrf_token = eldoria_encryption::hash_session_token(&csrf_token_raw, secret)?;

        let ip_addr = user_device.ip.map(|n| n.to_string());

        let create_session = CreateSession {
            id: self.snowflake.generate(),
            user_id,
            token: session_token,
            csrf: csrf_token,
            expires_at,
            ip_address: ip_addr,
            user_agent: Some(user_device.raw_user_agent.clone()),
        };

        let session = self.storage.create_session(&create_session).await?;

        Ok(SessionCreated {
            session,
            raw_token: session_token_raw,
            raw_csrf_token: csrf_token_raw,
        })
    }

    pub async fn validate_session(
        &self,
        token: &str,
        server_secret: &SecretString,
    ) -> Result<ValidatedSession, AuthError> {
        let secret = server_secret.expose_secret().as_bytes();
        let session_token = eldoria_encryption::hash_session_token(token, secret)?;

        let session = self
            .storage
            .get_session_by_token(&session_token)
            .await?
            .ok_or(AuthError::InvalidSession)?;

        if session.revoked_at.is_some() {
            return Err(AuthError::Revoked);
        }

        let now = Utc::now();
        if now >= session.expires_at {
            return Err(AuthError::Expired);
        }

        // Determine if we should rotate/extend based on policy
        let should_rotate = match self.config.sliding_policy {
            SlidingPolicy::Fixed => false,
            SlidingPolicy::Always => true,
            SlidingPolicy::Window(window_secs) => {
                let remaining = session.expires_at - now;
                remaining < chrono::Duration::seconds(window_secs)
            }
        };

        let mut new_token = None;

        if should_rotate {
            let new_expires_at = Some(now + chrono::Duration::seconds(self.config.ttl_seconds));
            let new_token_raw = generate_token(self.config.token_length)?;
            let new_token_hash = eldoria_encryption::hash_session_token(&new_token_raw, secret)?;

            self.storage
                .rotate_session_token(
                    session.id,
                    &new_token_hash,
                    &session_token,
                    session.expires_at,
                    new_expires_at,
                )
                .await?;

            new_token = Some(new_token_raw);
        }

        Ok(ValidatedSession {
            session,
            new_token, // Awlays `None` for Fixed policy
            new_csrf_token: None,
        })
    }

    pub async fn revoke_session(&self, id: Snowflake) -> Result<(), AuthError> {
        self.storage.revoke_session(id).await?;
        Ok(())
    }

    pub fn verify_csrf(
        &self,
        session: &Session,
        raw_csrf_token: &str,
        server_secret: &SecretString,
    ) -> Result<(), AuthError> {
        let secret = server_secret.expose_secret().as_bytes();
        let hash = eldoria_encryption::hash_session_token(raw_csrf_token, secret)?;

        if hash != session.csrf_token {
            return Err(AuthError::InvalidCsrfToken);
        }

        Ok(())
    }

    pub async fn maybe_rotate_csrf(
        &self,
        session: &Session,
        server_secret: &SecretString,
    ) -> Result<Option<String>, AuthError> {
        // Increment first so AfterRequests has an accurate count
        let request_count = self.storage.increment_request_count(session.id).await?;

        let now = Utc::now();

        let should_rotate = match self.config.csrf_rotation_policy {
            CsrfRotationPolicy::Never => false,
            CsrfRotationPolicy::Periodic(period_secs) => {
                let elapsed = now - session.last_csrf_rotated_at;
                elapsed > chrono::Duration::seconds(period_secs)
            }
            CsrfRotationPolicy::AfterRequests(max_requests) => request_count >= max_requests as i32,
        };

        if !should_rotate {
            return Ok(None);
        }

        let secret = server_secret.expose_secret().as_bytes();
        let new_csrf_raw = generate_token(self.config.csrf_token_length)?;
        let new_csrf_hash = eldoria_encryption::hash_session_token(&new_csrf_raw, secret)?;

        self.storage
            .rotate_csrf_token(session.id, &new_csrf_hash)
            .await?;

        Ok(Some(new_csrf_raw))
    }
}

fn generate_token(length: usize) -> Result<String, AuthError> {
    let mut random_bytes = vec![0u8; length];
    SysRng
        .try_fill_bytes(&mut random_bytes)
        .map_err(|_| AuthError::TokenGenerationFailed)?;
    Ok(URL_SAFE_NO_PAD.encode(&random_bytes))
}
