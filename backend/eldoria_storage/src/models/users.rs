use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::prelude::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Snowflake,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateUser {
    pub id: Snowflake,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub password_hash: String,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateUser {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub password_hash: Option<String>,
}
