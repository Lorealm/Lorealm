use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

/// A WTN document with unbound inputs. In other words, a constructor for realms.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Blueprint {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    /// The WTN source. Parsed on save, expanded on use.
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A blueprint without its source, for listings.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BlueprintSummary {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    pub updated_at: DateTime<Utc>,
}

/// Where a realm came from, when it came from a blueprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmOrigin {
    pub blueprint_id: Snowflake,
    #[serde(default)]
    pub inputs: std::collections::HashMap<String, serde_json::Value>,
}
