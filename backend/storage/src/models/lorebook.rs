use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Lorebook {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub current_version_id: Snowflake,
    pub name: String,
    pub description: String,
    pub config_yaml: String,
    pub schema_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub entries: Vec<LorebookEntry>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct LorebookEntry {
    pub id: Snowflake,
    pub lorebook_id: Snowflake,
    pub entry_id: String,
    pub source: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct LorebookVersionInfo {
    pub id: Snowflake,
    pub version_num: i32,
    pub book_version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LorebookMetadata {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub description: String,
    pub entry_count: usize,
}

/// Pull a content version label out of the lorebook config YAML.
/// Looks for a top-level `version:` key; defaults to "0".
pub fn extract_book_version(config_yaml: &str) -> String {
    serde_yaml::from_str::<serde_yaml::Value>(config_yaml)
        .ok()
        .and_then(|v| {
            let f = v.get("version")?;
            f.as_str()
                .map(str::to_owned)
                .or_else(|| f.as_u64().map(|n| n.to_string()))
        })
        .unwrap_or_else(|| "0".to_string())
}

impl From<&Lorebook> for LorebookMetadata {
    fn from(value: &Lorebook) -> Self {
        Self {
            id: value.id,
            owner_id: value.owner_id,
            name: value.name.clone(),
            description: value.description.clone(),
            entry_count: value.entries.len(),
        }
    }
}
