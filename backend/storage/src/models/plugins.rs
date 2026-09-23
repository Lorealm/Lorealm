use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

/// A plugin a user has enabled, at the version they're on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPlugin {
    pub handle: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PluginLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Who wrote a [`PluginLog`]: the server, or the plugin itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PluginLogSource {
    Host,
    Guest,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PluginLog {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub handle: String,
    pub level: PluginLogLevel,
    pub source: PluginLogSource,
    pub message: String,
    pub created_at: DateTime<Utc>,
}
