use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::prelude::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_uri: Option<String>,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub id: Snowflake,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUser {
    pub email: Option<String>,
    /// `Some(None)` clears the display name
    pub display_name: Option<Option<String>>,
    pub avatar_uri: Option<String>,
    pub password_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Invite {
    pub id: Snowflake,
    pub code_hash: String,
    /// Given to whoever registers with the code.
    pub role_id: Option<Snowflake>,
    pub created_by: Option<Snowflake>,
    /// `None` means unlimited.
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateInvite {
    pub id: Snowflake,
    pub code_hash: String,
    pub role_id: Option<Snowflake>,
    pub created_by: Option<Snowflake>,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
}
