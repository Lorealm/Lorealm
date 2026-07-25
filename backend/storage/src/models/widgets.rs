use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWidget {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub name: String,
    pub description: String,
    pub source: String,
    pub prompt: String,
    pub tools: Vec<Snowflake>,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
