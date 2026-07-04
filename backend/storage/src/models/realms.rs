use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::{ChatRole, Snowflake};

pub struct AgentSpec {
    pub owner: Snowflake,
    pub character_id: Snowflake,
    pub char_version_id: Snowflake,
    pub template_id: Snowflake,
    pub persona_id: Snowflake,
    pub tokenizer_id: Snowflake,
    pub profile_id: Snowflake,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RealmStatus {
    Active,
    Paused,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Realm {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub tick: i64,
    pub state: serde_json::Value,
    pub tokenizer_id: Snowflake,
    pub status: RealmStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Llm,
    Player,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RealmAgent {
    pub id: Snowflake,
    pub realm_id: Snowflake,
    pub name: String,
    pub kind: AgentKind,

    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub user_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,
    pub turn_order: i32,
    pub extensions: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentGrant {
    pub agent_id: Snowflake,
    pub scope: String,
    pub access: AgentAccess,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RealmTick {
    pub realm_id: Snowflake,
    pub tick: i64,
    pub agent_id: Option<Snowflake>,
    pub delta: serde_json::Value,
    pub snapshot: Option<serde_json::Value>,
}

pub struct TickCommit {
    pub realm_id: Snowflake,
    pub expected_tick: i64,
    pub agent_id: Option<Snowflake>,
    pub delta: serde_json::Value,
    pub snapshot: Option<serde_json::Value>,
    pub outcome: serde_json::Value,
    pub new_state: serde_json::Value,
    pub events: Vec<RealmEvent>,
    pub observations: Vec<AgentObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Spoke,
    Acted,
    StateChanged,
    System,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RealmEvent {
    pub id: Snowflake,
    pub realm_id: Snowflake,
    pub tick: i64,
    pub seq: i32,
    pub source: Option<Snowflake>,
    pub scope: String,
    pub kind: EventKind,
    pub content: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentObservation {
    pub id: Snowflake,
    pub agent_id: Snowflake,
    pub event_id: Option<Snowflake>,
    pub tick: i64,
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentAccess {
    Read,
    ReadWrite,
}

pub struct Grant {
    pub scope: ScopePattern,
    pub access: AgentAccess,
}

pub enum ScopePattern {
    Exact(String),
    Prefix(String),
}

impl ScopePattern {
    pub fn matches(&self, ns: &str) -> bool {
        match self {
            ScopePattern::Exact(s) => ns == s,
            ScopePattern::Prefix(p) => ns.starts_with(p),
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.strip_suffix('*') {
            Some(p) => ScopePattern::Prefix(p.to_string()),
            None => ScopePattern::Exact(s.to_string()),
        }
    }

    pub fn as_str(&self) -> String {
        match self {
            ScopePattern::Exact(s) => s.clone(),
            ScopePattern::Prefix(p) => format!("{p}*"),
        }
    }
}
