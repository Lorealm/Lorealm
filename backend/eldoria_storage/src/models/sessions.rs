use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::prelude::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub token: String,
    pub csrf: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSession {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub token: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub expires_at: DateTime<Utc>,
}
