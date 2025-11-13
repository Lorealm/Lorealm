use anyhow::anyhow;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres, types::Uuid, Row};
use std::error::Error;
use std::{fmt, fs};
use std::path::PathBuf;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::SaltString;
use argon2::password_hash::rand_core::OsRng as ArgonRng;
use rand::rand_core::{TryRngCore, OsRng};

use crate::routes::csrf::create_csrf_token;

#[derive(Debug)]
pub enum SessionError {
    NoTokenFound(String),
    TokenMismatch(String),
    TokenRetrievalFailed(String),
    Database(sqlx::Error),
    TechincalError(String)
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SessionError::NoTokenFound(msg) => write!(f, "{}", msg),
            SessionError::TokenMismatch(msg) => write!(f, "{}", msg),
            SessionError::TokenRetrievalFailed(msg) => write!(f, "{}", msg),
            SessionError::Database(err) => write!(f, "Database error: {}", err),
            SessionError::TechincalError(msg) => write!(f, "Technical error: {}", msg)
        }
    }
}

impl Error for SessionError {}

impl From<sqlx::Error> for SessionError {
    fn from(err: sqlx::Error) -> Self {
        SessionError::Database(err)
    }
}

// Ensure SessionError is Send
unsafe impl Send for SessionError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum Role {
    System = 2,
    Admin = 1,
    User = 0,
}

impl Role {
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s.to_lowercase().as_str() {
            "system" => Ok(Role::System),
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err("Invalid role"),
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            Role::System => "system".to_string(),
            Role::Admin => "admin".to_string(),
            Role::User => "user".to_string(),
        }
    }
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub email: Option<String>,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub role: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub password: String,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserSettings {
    pub theme: String,
    pub default_model: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub system_prompt: Option<String>,
    pub auto_save: bool,
    pub conversation_retention_days: u32,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            default_model: None,
            max_tokens: 2048,
            temperature: 0.7,
            system_prompt: None,
            auto_save: true,
            conversation_retention_days: 30,
        }
    }
}

pub struct UserManager {
    db: Pool<Postgres>,
    data_dir: PathBuf,
}

impl UserManager {
    pub fn new(db: Pool<Postgres>, data_dir: PathBuf) -> Self {
        Self { db, data_dir }
    }

    pub async fn create_user(&self, req: CreateUserRequest, creator_role: &Role, id_override: Option<Uuid>) -> Result<User, Box<anyhow::Error>> {
        // Only admins can create users
        if *creator_role < Role::Admin {
            return Err(Box::new(anyhow!("Only admins can create users")));
        }

        // Hash password
        let salt = SaltString::generate(&mut ArgonRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| anyhow!(e))?
            .to_string();

        let role = req.role.unwrap_or_else(|| "user".to_string());
        let _ = Role::from_str(&role).map_err(|e| anyhow!(e)); // Validate role

        let uuid = id_override.unwrap_or_else(Uuid::new_v4);

        // Insert user into database
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (id, username, email, password_hash, role)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#
        )
        .bind(sqlx::types::Uuid::from(uuid))
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&role)
        .fetch_one(&self.db)
        .await
        .map_err(|e| anyhow!(e))?;

        // Create user directory structure
        self.create_user_directory(user.id.to_string()).await
            .map_err(|_| anyhow::Error::msg("Error creating user directory"))?;

        Ok(user)
    }

    pub async fn create_root_user(&self) -> Result<(), Box<anyhow::Error>> {
        let req = CreateUserRequest {
            username: "root".to_string(),
            email: None,
            password: "password".to_string(),
            role: Some("admin".to_string()),
        };
        // Check if root user exists in database
        let root_user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1"
        ).bind(&req.username)
        .fetch_optional(&self.db)
        .await
        .map_err(|e| anyhow!(e))?;

        if root_user.is_some() {
            log::info!("Root user found in database");
            return Ok(());
        }

        log::info!("Root user not found in database, creating root user");
        self.create_user(
            req,
            &Role::System,
            Some(Uuid::nil())
        ).await?;

        return Ok(());
    }

    pub async fn authenticate(&self, req: LoginRequest) -> Result<(User, String, String), SessionError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = $1 AND is_active = true"
        )
        .bind(&req.username)
        .fetch_optional(&self.db)
        .await?
        .ok_or(SessionError::TokenRetrievalFailed("Invalid user".into()))?;

        // Verify password
        let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e| SessionError::TechincalError(e.to_string()))?;
        Argon2::default().verify_password(req.password.as_bytes(), &parsed_hash).map_err(|e| SessionError::TechincalError(e.to_string()))?;

        // Create session token
        let session_token = generate_session_token();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        let inserted_id: i32 = sqlx::query(
            r#"
            INSERT INTO user_sessions (user_id, session_token, expires_at)
            VALUES ($1, $2, $3)
            RETURNING id
            "#
        )
        .bind(&user.id.clone())
        .bind(&session_token)
        .bind(expires_at)
        .fetch_one(&self.db)
        .await?
        .get::<i32, _>(0);

        let initial_csrf = create_csrf_token(inserted_id, &self.db).await
            .map_err(|e| SessionError::TechincalError(e.to_string()))?;

        // Update last login
        sqlx::query("UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = $1")
            .bind(&user.id)
            .execute(&self.db)
            .await?;

        Ok((user, session_token, initial_csrf))
    }

    pub async fn validate_session(&self, token: &str) -> Result<User, SessionError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.* FROM users u
            JOIN user_sessions s ON u.id = s.user_id
            WHERE s.session_token = $1 AND s.expires_at > CURRENT_TIMESTAMP AND u.is_active = true
            "#
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?
        .ok_or(SessionError::TokenMismatch("Invalid or expired session".to_string()))?;

        Ok(user)
    }

    pub async fn get_session_id(&self, token: &str) -> Result<i32, SessionError> {
        let session_id = sqlx::query("SELECT id FROM user_sessions WHERE session_token = $1")
            .bind(token)
            .fetch_one(&self.db)
            .await?
            .get(0);
        Ok(session_id)
    }

    pub async fn logout(&self, token: &str) -> Result<(), SessionError> {
        sqlx::query("DELETE FROM user_sessions WHERE session_token = $1")
            .bind(token)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    async fn create_user_directory(&self, user_id: String) -> Result<(), Box<dyn std::error::Error>> {
        let user_dir = self.data_dir.join("users").join(user_id.to_string());
        
        fs::create_dir_all(&user_dir)?;
        fs::create_dir_all(user_dir.join("conversations"))?;
        fs::create_dir_all(user_dir.join("lorebooks"))?;
        fs::create_dir_all(user_dir.join("plugins"))?;
        fs::create_dir_all(user_dir.join("exports"))?;

        // Create default settings
        let settings = UserSettings::default();
        let settings_path = user_dir.join("settings.json");
        fs::write(settings_path, serde_json::to_string_pretty(&settings)?)?;

        Ok(())
    }

    pub fn get_user_settings(&self, user_id: String) -> Result<UserSettings, Box<dyn std::error::Error>> {
        let settings_path = self.data_dir
            .join("users")
            .join(user_id.to_string())
            .join("settings.json");

        if !settings_path.exists() {
            return Ok(UserSettings::default());
        }

        let content = fs::read_to_string(settings_path)?;
        let settings: UserSettings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub async fn migrate_token(&self, old_token: &str) -> Result<String, Box<dyn std::error::Error + Send>> {
        let token = sqlx::query(
            r#"
            SELECT session_token
            FROM user_sessions
            WHERE expires_at < now()
            AND expires_at > now() - interval '24 hours'
            ORDER BY expires_at DESC
            LIMIT 1;
            "#
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|e| Box::new(SessionError::Database(e)) as Box<dyn std::error::Error + Send>)?
        .ok_or_else(|| Box::new(SessionError::NoTokenFound("No token found".to_string())) as Box<dyn std::error::Error + Send>)?;

        if let Ok(session_token) = token.try_get::<String, _>(0) {
            if old_token == session_token {
                // Automatic refresh: Clear all old tokens
                sqlx::query(
                    r#"
                    DELETE FROM user_sessions
                    WHERE expires_at <= now()
                    OR session_token = $1
                    "#
                )
                .bind(old_token)
                .execute(&self.db)
                .await
                .map_err(|e| Box::new(SessionError::Database(e)) as Box<dyn std::error::Error + Send>)?;

                let new_token = generate_session_token();
                let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

                // Insert new token
                sqlx::query(
                    r#"
                    INSERT INTO user_sessions (session_token, expires_at)
                    VALUES ($1, $2)
                    RETURNING session_token
                    "#
                )
                .bind(&new_token)
                .bind(expires_at)
                .fetch_one(&self.db)
                .await
                .map_err(|e| Box::new(SessionError::Database(e)) as Box<dyn std::error::Error + Send>)?;

                Ok(new_token)
            } else {
                Err(Box::new(SessionError::TokenMismatch("Token mismatch".to_string())) as Box<dyn std::error::Error + Send>)
            }
        } else {
            Err(Box::new(SessionError::TokenRetrievalFailed("Failed to retrieve session token".to_string())) as Box<dyn std::error::Error + Send>)
        }
    }
    pub fn update_user_settings(&self, user_id: String, settings: UserSettings) -> Result<(), Box<dyn std::error::Error>> {
        let settings_path = self.data_dir
            .join("users")
            .join(user_id.to_string())
            .join("settings.json");

        fs::write(settings_path, serde_json::to_string_pretty(&settings)?)?;
        Ok(())
    }

    pub fn get_user_directory(&self, user_id: i64) -> PathBuf {
        self.data_dir.join("users").join(user_id.to_string())
    }
}

fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    let _ = OsRng.try_fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(&bytes)
}