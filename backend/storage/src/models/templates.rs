use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Template {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    /// JSON-serialized nodes structure
    pub nodes_json: serde_json::Value,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Lightweight template metadata for listings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    pub node_count: usize,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
