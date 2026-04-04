use std::sync::Arc;

use eldoria_encryption::Hashable;
use eldoria_storage::{
    Storage,
    models::{CreateUser, User},
};
use secrecy::SecretString;

use crate::{
    config::UserConfig,
    errors::AuthError,
    extract::UserDevice,
    services::{session::SessionService, snowflake::SnowflakeService, users::LoginResult},
};

pub struct UserService {
    storage: Arc<dyn Storage>,
    snowflake: Arc<SnowflakeService>,
    config: UserConfig,
    session_service: Arc<SessionService>,
}

impl UserService {
    pub fn new(
        storage: Arc<dyn Storage>,
        snowflake: Arc<SnowflakeService>,
        config: UserConfig,
        session_service: Arc<SessionService>,
    ) -> Self {
        Self {
            storage,
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

        eldoria_encryption::verify_password(&password, &user.password_hash)
            .map_err(|_| AuthError::IncorrectPassword)?;

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
        if pw_length < self.config.password_length.0 || pw_length > self.config.password_length.1 {
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
            display_name: username,
            email: email,
            password_hash: pw_hash,
        };

        Ok(self.storage.create_user(&create_user).await?)
    }

    pub async fn get_user(
        &self,
        user_id: eldoria_storage::models::Snowflake,
    ) -> Result<User, AuthError> {
        if let Ok(Some(user)) = self.storage.get_user_by_id(user_id).await {
            return Ok(user);
        }
        return Err(AuthError::UserNotFound);
    }
}
