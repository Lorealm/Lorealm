use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CodeModule {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    pub source: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
