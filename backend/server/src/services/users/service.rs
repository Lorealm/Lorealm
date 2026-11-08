use std::sync::Arc;

use bytes::Bytes;
use lorealm_encryption::Hashable;
use lorealm_storage::{
    BlobStore, Storage,
    models::{CreateUser, Session, Snowflake, UpdateUser, User},
};
use secrecy::SecretString;

use crate::{
    config::UserConfig,
    errors::AuthError,
    extract::UserDevice,
    services::{session::SessionService, snowflake::SnowflakeService, users::LoginResult},
};

const MAX_DISPLAY_NAME_CHARS: usize = 64;

pub struct UserService {
    storage: Arc<dyn Storage>,
    blob: Arc<dyn BlobStore>,
    snowflake: Arc<SnowflakeService>,
    config: UserConfig,
    session_service: Arc<SessionService>,
}

impl UserService {
    pub fn new(
        storage: Arc<dyn Storage>,
        blob: Arc<dyn BlobStore>,
        snowflake: Arc<SnowflakeService>,
        config: UserConfig,
        session_service: Arc<SessionService>,
    ) -> Self {
        Self {
            storage,
            blob,
            snowflake,
            config,
            session_service,
        }
    }

    pub async fn login(
        &self,
        email: Option<String>,
        username: Option<String>,
        password: String,
        server_secret: &SecretString,
        user_device: UserDevice,
    ) -> Result<LoginResult, AuthError> {
        let user = if let Some(email) = email {
            self.storage.get_user_by_email(&email).await?
        } else if let Some(username) = username {
            self.storage.get_user_by_username(&username).await?
        } else {
            return Err(AuthError::MalformedCredentials);
        }
        .ok_or(AuthError::UserNotFound)?;

        let is_valid = lorealm_encryption::verify_password(&password, &user.password_hash)
            .map_err(|_| AuthError::IncorrectPassword)?;

        if !is_valid {
            return Err(AuthError::IncorrectPassword);
        }

        let session_created = self
            .session_service
            .create_session(user.id, server_secret, user_device)
            .await?;

        Ok(LoginResult {
            user_id: user.id,
            session_token: session_created.raw_token,
            csrf_token: session_created.raw_csrf_token,
            expires_at: session_created.session.expires_at,
        })
    }

    pub async fn register(
        &self,
        username: String,
        email: Option<String>,
        password: String,
    ) -> Result<User, AuthError> {
        let pw_length = password.len();
        if pw_length < self.config.password_length.min
            || pw_length > self.config.password_length.max
        {
            return Err(AuthError::MalformedCredentials);
        }

        if let Ok(Some(_)) = self.storage.get_user_by_username(&username).await {
            return Err(AuthError::UsernameTaken(username));
        }

        if let Some(email) = &email {
            if let Ok(Some(_)) = self.storage.get_user_by_email(email).await {
                return Err(AuthError::EmailTaken(email.clone()));
            }
        }

        let pw_hash = password.hash().await?;

        let create_user = CreateUser {
            id: self.snowflake.generate(),
            username: username.clone(),
            email: email,
            password_hash: pw_hash,
        };

        Ok(self.storage.create_user(&create_user).await?)
    }

    pub async fn get_user(
        &self,
        user_id: lorealm_storage::models::Snowflake,
    ) -> Result<User, AuthError> {
        if let Ok(Some(user)) = self.storage.get_user_by_id(user_id).await {
            return Ok(user);
        }
        return Err(AuthError::UserNotFound);
    }

    /// Rename a user. A blank name clears it.
    pub async fn set_display_name(
        &self,
        user_id: Snowflake,
        display_name: Option<String>,
    ) -> Result<User, AuthError> {
        let display_name = display_name.and_then(|name| {
            let trimmed = name.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_string())
        });

        if let Some(name) = &display_name {
            if name.chars().count() > MAX_DISPLAY_NAME_CHARS {
                return Err(AuthError::DisplayNameTooLong);
            }
        }

        let update = UpdateUser {
            display_name: Some(display_name),
            ..Default::default()
        };

        Ok(self.storage.update_user(user_id, &update).await?)
    }

    /// Change the user's password after checking the current one, then sign
    /// out every other session so a leaked password stops working everywhere.
    pub async fn change_password(
        &self,
        user_id: Snowflake,
        current_session: Snowflake,
        current_password: String,
        new_password: String,
    ) -> Result<(), AuthError> {
        let (min, max) = (
            self.config.password_length.min,
            self.config.password_length.max,
        );
        if !(min..=max).contains(&new_password.len()) {
            return Err(AuthError::PasswordLength { min, max });
        }

        let user = self
            .storage
            .get_user_by_id(user_id)
            .await?
            .ok_or(AuthError::UserNotFound)?;

        let is_valid = lorealm_encryption::verify_password(&current_password, &user.password_hash)
            .map_err(|_| AuthError::WrongCurrentPassword)?;
        if !is_valid {
            return Err(AuthError::WrongCurrentPassword);
        }

        let update = UpdateUser {
            password_hash: Some(new_password.hash().await?),
            ..Default::default()
        };
        self.storage.update_user(user_id, &update).await?;

        self.revoke_other_sessions(user_id, current_session).await?;
        Ok(())
    }

    pub async fn set_avatar(
        &self,
        user_id: Snowflake,
        image_bytes: Bytes,
    ) -> Result<String, AuthError> {
        let avatar_uri = self.blob.put(&image_bytes).await?.to_uri();

        let update = UpdateUser {
            avatar_uri: Some(avatar_uri.clone()),
            ..Default::default()
        };
        self.storage.update_user(user_id, &update).await?;

        Ok(avatar_uri)
    }

    /// The user's live sessions, newest first.
    pub async fn list_sessions(&self, user_id: Snowflake) -> Result<Vec<Session>, AuthError> {
        Ok(self.storage.list_user_sessions(user_id).await?)
    }

    /// Sign one device out.
    pub async fn revoke_session(
        &self,
        user_id: Snowflake,
        session_id: Snowflake,
    ) -> Result<(), AuthError> {
        let sessions = self.storage.list_user_sessions(user_id).await?;
        if !sessions.iter().any(|s| s.id == session_id) {
            return Err(AuthError::InvalidSession);
        }

        Ok(self.storage.revoke_session(session_id).await?)
    }

    pub async fn revoke_other_sessions(
        &self,
        user_id: Snowflake,
        keep: Snowflake,
    ) -> Result<u64, AuthError> {
        let sessions = self.storage.list_user_sessions(user_id).await?;

        let mut revoked = 0;
        for session in sessions.iter().filter(|s| s.id != keep) {
            self.storage.revoke_session(session.id).await?;
            revoked += 1;
        }

        Ok(revoked)
    }
}
