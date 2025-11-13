use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Duration, Utc};
use rand::{TryRngCore, rngs::OsRng};
use storage_system::{Storage, StorageError};
use uuid::Uuid;

use crate::{
    models::{SessionService, UserDevice},
    services::authentication::error::SessionError,
};

/// Defines how session expiration should be extended on activity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlidingPolicy {
    /// Never extend expiration - session has fixed lifetime from creation
    Fixed,

    /// Always extend expiration on every activity
    Always,

    /// Only extend if session is within the specified window before expiry
    Window(i64),
}

impl Default for SlidingPolicy {
    fn default() -> Self {
        Self::Always // Sensible default for most use cases
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CsrfRotationPolicy {
    /// Never rotate - token lasts for session lifetime
    /// Best for: SPAs, most applications
    Never,

    /// Rotate after every state-changing request
    /// Best for: High-security server-rendered apps
    /// Warning: Breaks concurrent requests
    PerRequest,

    /// Rotate every N seconds
    /// Best for: Balanced security/usability
    Periodic(i64),

    /// Rotate after N state-changing requests
    /// Best for: Rate-limited rotation
    AfterRequests(u32),
}

impl Default for CsrfRotationPolicy {
    fn default() -> Self {
        Self::Never
    }
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    pub token_length: usize,
    pub csrf_token_length: usize,
    pub ttl_seconds: i64,
    pub sliding_policy: SlidingPolicy,
    pub csrf_rotation_policy: CsrfRotationPolicy,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            token_length: 32,
            csrf_token_length: 32,
            ttl_seconds: 60 * 60 * 24, // 24 hours
            sliding_policy: SlidingPolicy::Always,
            csrf_rotation_policy: CsrfRotationPolicy::Never,
        }
    }
}

#[derive(Clone)]
pub struct Session {
    id: Uuid,
    user_id: Uuid,
    raw_token: Option<String>,
    session_token: String,
    raw_csrf_token: Option<String>,
    csrf_token: String,
    pub csrf_rotation_count: u32, // Track rotations for AfterRequests policy
    pub csrf_last_rotated_at: DateTime<Utc>, // Track time for Periodic policy
    ip_address: String,
    user_agent: String,
    added_at: DateTime<Utc>,
    last_active_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
    pub config: SessionConfig, // Add config to the session
}

impl Session {
    pub async fn new(
        user_id: Uuid,
        user_device: &UserDevice,
        server_secret: &str,
        config: SessionConfig,
    ) -> Result<Self, SessionError> {
        let _ = user_device.validate()?;
        let token = generate_session_token(config.token_length)?;
        let csrf_token = generate_session_token(config.csrf_token_length)?;
        let now = Utc::now();

        Ok(Self {
            id: Uuid::new_v4(),
            user_id,
            raw_token: Some(token.to_string()),
            session_token: encryption_service::hash_session_token(
                &token,
                server_secret.as_bytes(),
            )?,
            raw_csrf_token: Some(csrf_token.to_string()),
            csrf_token: encryption_service::hash_session_token(
                &csrf_token,
                server_secret.as_bytes(),
            )?,
            csrf_rotation_count: 0,
            csrf_last_rotated_at: now,
            ip_address: user_device.ip.clone(),
            user_agent: user_device.user_agent.clone(),
            added_at: now,
            last_active_at: now,
            expires_at: now + Duration::seconds(config.ttl_seconds),
            revoked_at: None,
            config,
        })
    }

    pub fn user_id(&self) -> Uuid {
        self.user_id
    }

    pub fn raw_token(&self) -> Option<String> {
        self.raw_token.clone()
    }

    pub fn raw_csrf_token(&self) -> Option<String> {
        self.raw_csrf_token.clone()
    }

    /// Retrieves a session object from the database. Performs cryptographic validation.
    pub async fn retrieve<S>(
        session_token: &str,
        server_secret: &str,
        session_service: &SessionService<S>,
    ) -> Result<Self, SessionError>
    where
        S: Storage + ?Sized,
    {
        let computed =
            encryption_service::hash_session_token(session_token, server_secret.as_bytes())?;
        let session = session_service.get_session(&computed).await?;

        if !encryption_service::verify_session_token(&computed, &session.session_token)? {
            return Err(SessionError::InvalidToken);
        }

        Ok(session)
    }

    /// Invalidates existing sessions with the same user id, ip and user agents
    /// before creating a new one
    pub async fn invalidate_and_create<S>(
        user_id: Uuid,
        user_device: &UserDevice,
        server_secret: &str,
        config: SessionConfig,
        session_service: &SessionService<S>,
    ) -> Result<Self, SessionError>
    where
        S: Storage + ?Sized,
    {
        session_service.invalidate_sessions(user_id, user_device).await?;
        Ok(Self::new(user_id, user_device, server_secret, config).await?)
    }

    /// Validates CSRF token for state-changing requests
    pub fn verify_csrf_token(
        &self,
        provided_token: &str,
        server_secret: &str,
    ) -> Result<(), SessionError> {
        let computed =
            encryption_service::hash_session_token(provided_token, server_secret.as_bytes())?;

        if !encryption_service::verify_session_token(&computed, &self.csrf_token)? {
            return Err(SessionError::InvalidCsrfToken);
        }

        Ok(())
    }

    pub async fn rotate_csrf<S>(
        &mut self,
        server_secret: &str,
        session_service: &SessionService<S>,
    ) -> Result<String, SessionError>
    where
        S: Storage + ?Sized,
    {
        let new_csrf = generate_session_token(self.config.csrf_token_length)?;
        let hashed_csrf =
            encryption_service::hash_session_token(&new_csrf, server_secret.as_bytes())?;

        self.csrf_token = hashed_csrf.clone();
        self.csrf_rotation_count += 1;
        self.csrf_last_rotated_at = Utc::now();

        // Update in database
        session_service
            .update_csrf_token(
                self.id,
                hashed_csrf,
                self.csrf_rotation_count,
                self.csrf_last_rotated_at,
            )
            .await?;

        Ok(new_csrf)
    }

    /// Checks if CSRF should be rotated based on policy
    pub fn should_rotate_csrf(&self) -> bool {
        match self.config.csrf_rotation_policy {
            CsrfRotationPolicy::Never => false,
            CsrfRotationPolicy::PerRequest => true,
            CsrfRotationPolicy::Periodic(seconds) => {
                let elapsed = Utc::now() - self.csrf_last_rotated_at;
                elapsed.num_seconds() >= seconds
            }
            CsrfRotationPolicy::AfterRequests(count) => self.csrf_rotation_count >= count,
        }
    }

    /// Bumps the session token's `last_active_at` property
    pub async fn bump<S>(&mut self, session_service: &SessionService<S>) -> Result<(), SessionError>
    where
        S: Storage + ?Sized,
    {
        let now = Utc::now();
        self.last_active_at = now;

        let new_expiration = match self.config.sliding_policy {
            SlidingPolicy::Fixed => {
                // Never extend expiration
                None
            }

            SlidingPolicy::Always => {
                // Always extend expiration
                Some(now + Duration::seconds(self.config.ttl_seconds))
            }

            SlidingPolicy::Window(window_seconds) => {
                // Only extend if within the sliding window
                let time_until_expiry = self.expires_at - now;

                if time_until_expiry.num_seconds() < window_seconds {
                    // Close to expiring - slide the window
                    Some(now + Duration::seconds(self.config.ttl_seconds))
                } else {
                    // Still plenty of time - don't update expiration
                    None
                }
            }
        };

        // Update expiration if policy dictates
        if let Some(new_expiry) = new_expiration {
            self.expires_at = new_expiry;
        }

        // Update database
        session_service
            .update_session_activity(self.id, now, new_expiration)
            .await?;

        Ok(())
    }

    pub async fn revoke<S>(&self, session_service: &SessionService<S>) -> Result<(), SessionError>
    where
        S: Storage + ?Sized,
    {
        Ok(session_service.revoke_session(self.id).await?)
    }

    /// Validates the session
    ///
    /// The session is invalid if:
    /// 1. It's expired
    /// 2. It's revoked
    /// 3. The requester is from a different IP, or a different user agent
    pub async fn validate(&self, device: &UserDevice) -> Result<(), SessionError> {
        if Utc::now() > self.expires_at {
            return Err(SessionError::SessionExpired);
        }
        if self.revoked_at.is_some() {
            return Err(SessionError::SessionRevoked);
        }
        if self.ip_address != device.ip || self.user_agent != device.user_agent {
            return Err(SessionError::DeviceMismatch);
        }
        Ok(())
    }
}

fn generate_session_token(length: usize) -> Result<String, SessionError> {
    let mut random_bytes = vec![0u8; length];
    OsRng
        .try_fill_bytes(&mut random_bytes)
        .map_err(|_| SessionError::TokenGenerationFailed)?;
    Ok(URL_SAFE_NO_PAD.encode(&random_bytes))
}

impl From<storage_system::Session> for Session {
    fn from(value: storage_system::Session) -> Self {
        Self {
            id: value.id,
            user_id: value.user_id,
            raw_token: None,
            session_token: value.token,
            raw_csrf_token: None,
            csrf_token: value.csrf_token,
            csrf_rotation_count: value.csrf_rotation_count,
            csrf_last_rotated_at: value.csrf_last_rotated_at,
            ip_address: value.ip_address,
            user_agent: value.user_agent,
            added_at: value.added_at,
            last_active_at: value.last_active_at,
            expires_at: value.expires_at,
            revoked_at: value.revoked_at,
            config: SessionConfig::default(),
        }
    }
}

impl Into<storage_system::Session> for Session {
    fn into(self) -> storage_system::Session {
        storage_system::Session {
            id: self.id,
            user_id: self.user_id,
            token: self.session_token,
            csrf_token: self.csrf_token,
            csrf_rotation_count: self.csrf_rotation_count,
            csrf_last_rotated_at: self.csrf_last_rotated_at,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            added_at: self.added_at,
            last_active_at: self.last_active_at,
            expires_at: self.expires_at,
            revoked_at: self.revoked_at,
        }
    }
}
