use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Pool, Postgres, };
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::password_hash::{rand_core::OsRng, SaltString};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Admin,
    User,
}

impl Role {
    pub fn from_str(s: &str) -> Result<Self, &'static str> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(Role::Admin),
            "user" => Ok(Role::User),
            _ => Err("Invalid role"),
        }
    }
    
    pub fn to_string(&self) -> String {
        match self {
            Role::Admin => "admin".to_string(),
            Role::User => "user".to_string(),
        }
    }
}

#[derive(Debug, FromRow, Serialize, Clone)]
pub struct User {
    pub id: String,
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

    pub async fn create_user(&self, req: CreateUserRequest, creator_role: &Role) -> Result<User, Box<anyhow::Error>> {
        // Only admins can create users
        if *creator_role != Role::Admin {
            return Err(Box::new(anyhow!("Only admins can create users")));
        }

        // Hash password
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| anyhow!(e))?
            .to_string();

        let role = req.role.unwrap_or_else(|| "user".to_string());
        Role::from_str(&role).map_err(|e| anyhow!(e)); // Validate role

        // Insert user into database
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, role)
            VALUES (?, ?, ?, ?)
            RETURNING *
            "#
        )
        .bind(&req.username)
        .bind(&req.email)
        .bind(&password_hash)
        .bind(&role)
        .fetch_one(&self.db)
        .await
        .map_err(|e| anyhow!(e))?;

        // Create user directory structure
        self.create_user_directory(user.id.clone()).await
            .map_err(|_| anyhow::Error::msg("Error creating user directory"))?;

        Ok(user)
    }

    pub async fn authenticate(&self, req: LoginRequest) -> Result<(User, String), Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            "SELECT * FROM users WHERE username = ? AND is_active = true"
        )
        .bind(&req.username)
        .fetch_optional(&self.db)
        .await?
        .ok_or("Invalid credentials")?;

        // Verify password
        let parsed_hash = PasswordHash::new(&user.password_hash).map_err(|e| anyhow!(e))?;
        Argon2::default().verify_password(req.password.as_bytes(), &parsed_hash).map_err(|e| anyhow!(e))?;

        // Create session token
        let session_token = Uuid::new_v4().to_string();
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(24);

        sqlx::query(
            r#"
            INSERT INTO user_sessions (user_id, session_token, expires_at)
            VALUES (?, ?, ?)
            "#
        )
        .bind(&user.id.clone())
        .bind(&session_token)
        .bind(expires_at)
        .execute(&self.db)
        .await?;

        // Update last login
        sqlx::query("UPDATE users SET last_login = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(&user.id)
            .execute(&self.db)
            .await?;

        Ok((user, session_token))
    }

    pub async fn validate_session(&self, token: &str) -> Result<User, Box<dyn std::error::Error>> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT u.* FROM users u
            JOIN user_sessions s ON u.id = s.user_id
            WHERE s.session_token = ? AND s.expires_at > CURRENT_TIMESTAMP AND u.is_active = true
            "#
        )
        .bind(token)
        .fetch_optional(&self.db)
        .await?
        .ok_or("Invalid or expired session")?;

        Ok(user)
    }

    pub async fn logout(&self, token: &str) -> Result<(), Box<dyn std::error::Error>> {
        sqlx::query("DELETE FROM user_sessions WHERE session_token = ?")
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