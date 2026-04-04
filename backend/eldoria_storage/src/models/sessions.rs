use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::prelude::Snowflake;

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct Session {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub token: String,
    pub csrf_token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub last_rotated_at: DateTime<Utc>,
    pub previous_token: Option<String>,
    pub grace_expires_at: Option<DateTime<Utc>>,
    pub last_csrf_rotated_at: DateTime<Utc>,
    pub requests_since_csrf_rotation: i32,
}

#[derive(Debug, Clone)]
pub struct CreateSession {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub token: String,
    pub csrf: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
}
