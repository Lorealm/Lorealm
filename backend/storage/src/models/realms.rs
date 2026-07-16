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
    /// Lorebooks feeding context assembly. `None` = the owner's active set;
    /// `Some` = exactly these ids.
    pub lorebook_ids: Option<Vec<Snowflake>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Realm {
    pub id: Snowflake,
    pub owner_id: Snowflake,
    pub current_version_id: Snowflake,
    pub definition: RealmDefinition,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealmDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// Which of the owner's resources exist inside the realm. Agents may only
    /// reference imported resources.
    #[serde(default)]
    pub imports: Vec<ImportRule>,
    /// The cast, run in the same order as defined in the array
    #[serde(default)]
    pub agents: Vec<RealmAgent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    Character,
    Lorebook,
    Template,
    Tool,
}

impl ImportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportKind::Character => "character",
            ImportKind::Lorebook => "lorebook",
            ImportKind::Template => "template",
            ImportKind::Tool => "tool",
        }
    }
}

/// Declares that a resource exists inside the realm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRule {
    pub kind: ImportKind,
    pub resource_id: Snowflake,
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

/// A realm's formatting settings. Every field is an override: `None` uses the
/// built-in default. Stored as nullable columns per realm.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealmConfig {
    #[serde(default)]
    pub speech_format: Option<String>,
    #[serde(default)]
    pub action_format: Option<String>,
    #[serde(default)]
    pub tool_result_format: Option<String>,
    #[serde(default)]
    pub state_change_format: Option<String>,
    #[serde(default)]
    pub state_write_format: Option<String>,
    #[serde(default)]
    pub state_delete_format: Option<String>,
}

/// Realm settings with every field resolved to a concrete value.
#[derive(Debug, Clone)]
pub struct ResolvedRealmConfig {
    pub speech_format: String,
    pub action_format: String,
    pub tool_result_format: String,
    pub state_change_format: String,
    pub state_write_format: String,
    pub state_delete_format: String,
}

impl Default for ResolvedRealmConfig {
    fn default() -> Self {
        Self {
            speech_format: "{name}: {content}".into(),
            action_format: "[{name} used {tool}]".into(),
            tool_result_format: "[{tool} result: {output}]".into(),
            state_change_format: "[state {scope} changed: {changes}]".into(),
            state_write_format: "{key} = {value}".into(),
            state_delete_format: "-{key}".into(),
        }
    }
}

impl ResolvedRealmConfig {
    /// Layer a realm's overrides over the built-in defaults.
    pub fn resolve(config: &RealmConfig) -> Self {
        let mut cfg = Self::default();
        cfg.apply(config);
        cfg
    }

    fn apply(&mut self, o: &RealmConfig) {
        if let Some(v) = &o.speech_format {
            self.speech_format = v.clone();
        }
        if let Some(v) = &o.action_format {
            self.action_format = v.clone();
        }
        if let Some(v) = &o.tool_result_format {
            self.tool_result_format = v.clone();
        }
        if let Some(v) = &o.state_change_format {
            self.state_change_format = v.clone();
        }
        if let Some(v) = &o.state_write_format {
            self.state_write_format = v.clone();
        }
        if let Some(v) = &o.state_delete_format {
            self.state_delete_format = v.clone();
        }
    }

    /// Another agent's speech in an observer's feed. `{name}`, `{content}`.
    pub fn speech(&self, name: &str, content: &str) -> String {
        self.speech_format
            .replace("{name}", name)
            .replace("{content}", content)
    }

    /// The content of an `Acted` event. `{name}`, `{tool}`.
    pub fn action(&self, name: &str, tool: &str) -> String {
        self.action_format
            .replace("{name}", name)
            .replace("{tool}", tool)
    }

    /// An actor's private note after a tool runs. `{tool}`, `{output}`.
    pub fn tool_result(&self, tool: &str, output: &str) -> String {
        self.tool_result_format
            .replace("{tool}", tool)
            .replace("{output}", output)
    }

    /// The content of a `StateChanged` event. `{scope}`, `{changes}`.
    pub fn state_change(&self, scope: &str, changes: &str) -> String {
        self.state_change_format
            .replace("{scope}", scope)
            .replace("{changes}", changes)
    }

    /// One written variable within a state change. `{key}`, `{value}`.
    pub fn state_write(&self, key: &str, value: &str) -> String {
        self.state_write_format
            .replace("{key}", key)
            .replace("{value}", value)
    }

    /// One deleted variable within a state change. `{key}`.
    pub fn state_delete(&self, key: &str) -> String {
        self.state_delete_format.replace("{key}", key)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Llm,
    Player,
    Script,
}

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

    /// Scoped read/write access to state namespaces.
    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    /// Bound tools. Empty = speak-only.
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
    /// The scope this agent's [`EventKind::Spoke`] events carry. `None` = the broadcast
    /// `main` channel
    #[serde(default)]
    pub speech_scope: Option<String>,
    /// Imported lorebooks feeding this agent's context assembly. Empty = no
    /// lorebook context.
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    /// Inline weaver source run on this agent's turn.
    #[serde(default)]
    pub script: Option<String>,
    /// Weaver expression gating whether the agent runs on its turn, evaluated
    /// against its granted read slice. `None` or truthy = runs; falsy = skip.
    /// An erroring expression does not run.
    #[serde(default)]
    pub run_condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGrant {
    pub scope: String,
    pub access: AgentAccess,
}

/// A tool bound to a realm agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolBinding {
    pub tool_id: Snowflake,
    /// Scope of the `Acted` event emitted when this tool runs. `None` =
    /// only the acting agent observes it.
    #[serde(default)]
    pub event_scope: Option<String>,
}

/// The cast pinned onto a conversation when it instantiates a realm.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmCast {
    pub realm_id: Snowflake,
    /// The realm version this playthrough was instantiated from.
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

/// What happened on a turn, anchored to the message that committed it.
/// `source` is a cast agent id.
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
/// produced it.
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

/// The realm-side rows of one committed turn
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_overrides_over_defaults() {
        let config = RealmConfig {
            speech_format: Some("<{name}> {content}".into()),
            ..Default::default()
        };

        let cfg = ResolvedRealmConfig::resolve(&config);

        assert_eq!(cfg.speech("A", "hi"), "<A> hi"); // override
        assert_eq!(cfg.action("A", "cast"), "[A used cast]"); // default
        assert_eq!(cfg.tool_result("cast", "ok"), "[cast result: ok]"); // default
    }
}
