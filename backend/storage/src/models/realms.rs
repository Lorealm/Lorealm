use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::{ChatRole, Snowflake};

pub struct AgentSpec {
    pub owner: Snowflake,
    pub character: Option<(Snowflake, Snowflake)>,
    pub template_id: Snowflake,
    pub persona_id: Option<Snowflake>,
    pub tokenizer_id: Snowflake,
    pub profile_id: Snowflake,
}

/// A realm is a versioned preset, same machinery as characters: a head row
/// pointing at the current of its immutable version snapshots. The
/// definition (name, description, cast) is a document inside the snapshot.
/// It carries no runtime state — not even a seed pool: the opening state is
/// emergent, written by the weaver expressions its lorebooks/tools run
/// during early turns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Realm {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub current_version_id: Snowflake,

    pub definition: RealmDefinition,
    /// Synced child table, not versioned (like character tags).
    pub tags: Vec<String>,
}

/// The versioned document: everything that defines what a playthrough of
/// this preset is.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealmDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The cast, sorted by (turn_order, id); round-robin runs over it.
    #[serde(default)]
    pub agents: Vec<RealmAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmVersionInfo {
    pub id: Snowflake,
    pub version_num: u32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RealmSummary {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub name: String,
    pub tags: Vec<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Llm,
    Player,
}

/// One cast member as authored in the realm definition. `id` is a stable
/// identity within the realm — events and observations reference it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmAgent {
    pub id: Snowflake,
    pub name: String,
    pub kind: AgentKind,

    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub user_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,
    pub turn_order: i32,

    /// What the agent can know: scoped access to state namespaces.
    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    /// What the agent can do. Empty = deliberately speak-only.
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGrant {
    pub scope: String,
    pub access: AgentAccess,
}

/// A tool bound to a realm agent: its action space, one entry per tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolBinding {
    pub tool_id: Snowflake,
    /// Scope of the `Acted` event emitted when this tool runs. `None` =
    /// private — only the acting agent observes its own action.
    #[serde(default)]
    pub event_scope: Option<String>,
}

/// The instantiation contract: the cast pinned onto the conversation when
/// it starts. Kept even though versions are immutable-ish, because
/// edit-session saves update the current version in place — the cast is
/// what guarantees preset edits never reach an in-flight playthrough.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmCast {
    pub realm_id: Snowflake,
    /// Which realm version this playthrough was instantiated from
    /// (provenance; may be absent on casts pinned before versioning).
    #[serde(default)]
    pub version_id: Option<Snowflake>,
    pub agents: Vec<RealmAgent>,
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

/// What happened on a turn, anchored to the message that committed it so
/// swipes/branches carry their own event history. `source` is a
/// cast agent id.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RealmEvent {
    pub id: Snowflake,
    pub conversation_id: Snowflake,
    pub message_id: Snowflake,
    pub seq: i32,
    pub source: Option<Snowflake>,
    pub scope: String,
    pub kind: EventKind,
    pub content: String,
    pub payload: serde_json::Value,
}

/// One entry of an agent's private feed, keyed by the message whose turn
/// produced it — an agent's knowledge along a path is exactly the rows of
/// that path's messages. `agent_id` is a cast agent id.
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct AgentObservation {
    pub id: Snowflake,
    pub conversation_id: Snowflake,
    pub message_id: Snowflake,
    pub agent_id: Snowflake,
    pub event_id: Option<Snowflake>,
    pub role: ChatRole,
    pub content: String,
}

/// The realm-side rows of one committed turn. The message itself and its
/// state delta are persisted through the ordinary chat pipeline; this only
/// carries what fans out from them.
pub struct TurnCommit {
    pub events: Vec<RealmEvent>,
    pub observations: Vec<AgentObservation>,
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

#[derive(Debug, Clone)]
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
