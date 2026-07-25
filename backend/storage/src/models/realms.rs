use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::{ChatRole, Snowflake};

/// The state namespace holding per-agent variables: an agent with handle `h`
/// owns `agents:h.*`.
pub const AGENTS_SCOPE: &str = "agents";

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
    /// Named openings; instantiation picks one (see [`Scenario`]).
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
}

/// A named opening for a realm, analogous to a character greeting: a seed
/// script committed as the turn-zero state delta, optional opening narration
/// (a `System` event on `main`), and per-agent property overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: Snowflake,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub seed_script: String,
    #[serde(default)]
    pub opening: Option<String>,
    #[serde(default)]
    pub agent_overrides: HashMap<String, HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportKind {
    Character,
    Lorebook,
    Template,
    Tool,
    Persona,
    ConnectionProfile,
    Widget,
}

impl ImportKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ImportKind::Character => "character",
            ImportKind::Lorebook => "lorebook",
            ImportKind::Template => "template",
            ImportKind::Tool => "tool",
            ImportKind::Persona => "persona",
            ImportKind::ConnectionProfile => "connection profile",
            ImportKind::Widget => "widget",
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

/// A realm's formatting settings.
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
    /// Stable weaver-addressable identity
    #[serde(default)]
    pub handle: String,
    pub kind: AgentKind,

    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub user_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,
    pub turn_order: i32,

    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
    /// Scoped read/write access to state namespaces.
    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    /// Bound tools. Empty = speak-only.
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
    /// The scope this agent's [`EventKind::Spoke`] events carry. `None` = the broadcast
    /// `main` channel
    #[serde(default)]
    pub speech_channel: Option<String>,
    /// Imported lorebooks feeding this agent's context assembly. Empty = no
    /// lorebook context.
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    /// Imported static display widgets shown when this agent is the viewer.
    #[serde(default)]
    pub widgets: Vec<Snowflake>,
    /// Inline weaver source run on this agent's turn.
    #[serde(default)]
    pub script: Option<String>,
    /// Weaver expression gating whether the agent runs on its turn, evaluated
    /// against its granted read slice. `None` or truthy = runs; falsy = skip.
    /// An erroring expression does not run.
    #[serde(default)]
    pub run_condition: Option<String>,
}

impl RealmAgent {
    /// The implicit grant every handled agent holds on its own subtree:
    /// `agents:<handle>.*`. `None` for legacy agents without a handle.
    pub fn own_scope(&self) -> Option<ScopePattern> {
        (!self.handle.is_empty()).then(|| ScopePattern {
            scope: AGENTS_SCOPE.to_string(),
            name: NamePattern::Under(format!("{}.", self.handle)),
        })
    }
}

impl AgentKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentKind::Llm => "llm",
            AgentKind::Player => "player",
            AgentKind::Script => "script",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentGrant {
    pub channel: String,
    pub access: AgentAccess,
}

/// A tool bound to a realm agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToolBinding {
    pub tool_id: Snowflake,
    /// The channel of the `Acted` event emitted when this tool runs. `None` =
    /// only the acting agent observes it.
    #[serde(default)]
    pub event_channel: Option<String>,
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

/// What happened on a turn
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct RealmEvent {
    pub id: Snowflake,
    pub conversation_id: Snowflake,
    pub message_id: Snowflake,
    pub seq: i32,
    pub source: Option<Snowflake>,
    pub channel: String,
    pub kind: EventKind,
    pub content: String,
    pub payload: serde_json::Value,
}

/// One entry of an agent's private feed
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

/// A parsed grant in weaver `scope:name` syntax.
#[derive(Debug, Clone)]
pub struct ScopePattern {
    pub scope: String,
    pub name: NamePattern,
}

#[derive(Debug, Clone)]
pub enum NamePattern {
    Any,
    Exact(String),
    Under(String),
}

impl ScopePattern {
    /// Whether this grant covers the variable `scope:name`.
    pub fn matches_var(&self, scope: &str, name: &str) -> bool {
        self.scope == scope
            && match &self.name {
                NamePattern::Any => true,
                NamePattern::Exact(n) => name == n,
                NamePattern::Under(prefix) => name.starts_with(prefix.as_str()),
            }
    }

    /// Whether this grant observes events on `channel`
    pub fn matches_channel(&self, channel: &str) -> bool {
        self.scope == channel
    }

    pub fn parse(s: &str) -> Self {
        let (scope, name) = match s.split_once(':') {
            None => (s, None),
            Some((scope, name)) => (scope, Some(name)),
        };
        let name = match name {
            None | Some("*") => NamePattern::Any,
            Some(n) => match n.strip_suffix(".*") {
                Some(path) => NamePattern::Under(format!("{path}.")),
                None => NamePattern::Exact(n.to_string()),
            },
        };
        Self {
            scope: scope.to_string(),
            name,
        }
    }

    /// Reject grant strings outside the weaver `scope[:name]` form. [`parse`]
    /// itself stays lenient so grants stored before this rule keep matching
    /// as opaque strings.
    ///
    /// [`parse`]: ScopePattern::parse
    pub fn validate(s: &str) -> Result<(), String> {
        let (scope, name) = match s.split_once(':') {
            None => (s, None),
            Some((scope, name)) => (scope, Some(name)),
        };
        if !is_identifier(scope) {
            return Err(format!(
                "grant scope '{scope}' must be a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
            ));
        }
        match name {
            None | Some("*") => Ok(()),
            Some(n) => {
                let path = n.strip_suffix(".*").unwrap_or(n);
                if !path.is_empty() && path.split('.').all(is_identifier) {
                    Ok(())
                } else {
                    Err(format!(
                        "grant name '{n}' must be `*`, a dotted path, or a dotted path ending in `.*`"
                    ))
                }
            }
        }
    }

    pub fn as_str(&self) -> String {
        match &self.name {
            NamePattern::Any => self.scope.clone(),
            NamePattern::Exact(n) => format!("{}:{n}", self.scope),
            NamePattern::Under(prefix) => format!("{}:{prefix}*", self.scope),
        }
    }
}

/// Whether `s` is a weaver identifier: `[A-Za-z_][A-Za-z0-9_]*`.
pub fn is_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_scope_grants_every_var_in_the_pool() {
        let g = ScopePattern::parse("nation");
        assert!(g.matches_var("nation", "a.gold"));
        assert!(g.matches_var("nation", "b"));
        assert!(!g.matches_var("market", "a.gold"));
    }

    #[test]
    fn star_name_equals_bare_scope() {
        let g = ScopePattern::parse("nation:*");
        assert!(g.matches_var("nation", "a.gold"));
        assert!(!g.matches_var("market", "ore"));
    }

    #[test]
    fn dotted_star_matches_only_under_the_path() {
        let g = ScopePattern::parse("nation:a.*");
        assert!(g.matches_var("nation", "a.gold"));
        assert!(g.matches_var("nation", "a.army.size"));
        assert!(!g.matches_var("nation", "a")); // the path itself needs `nation:a`
        assert!(!g.matches_var("nation", "ab.gold")); // `.` bounds the prefix
        assert!(!g.matches_var("nation", "b.gold"));
    }

    #[test]
    fn exact_name_matches_one_var() {
        let g = ScopePattern::parse("market:ore.price");
        assert!(g.matches_var("market", "ore.price"));
        assert!(!g.matches_var("market", "ore.price.high"));
        assert!(!g.matches_var("market", "ore"));
    }

    #[test]
    fn channels_match_on_the_scope_half() {
        assert!(ScopePattern::parse("market").matches_channel("market"));
        assert!(ScopePattern::parse("market:ore.*").matches_channel("market"));
        assert!(!ScopePattern::parse("nation").matches_channel("market"));
    }

    #[test]
    fn validate_accepts_weaver_grant_forms() {
        for ok in [
            "nation",
            "nation:*",
            "nation:a",
            "nation:a.gold",
            "nation:a.*",
            "_x:y_2.z",
        ] {
            assert!(ScopePattern::validate(ok).is_ok(), "{ok}");
        }
    }

    #[test]
    fn validate_rejects_non_weaver_forms() {
        for bad in [
            "nation.a",       // dotted scope: pre-convergence form
            "market.*",       // scope-half wildcard
            "nation:",        // empty name
            ":gold",          // empty scope
            "nation:a..b",    // empty path segment
            "nation:*.a",     // wildcard must be trailing
            "1nation",        // not an identifier
            "nation:a.gold.", // trailing dot
        ] {
            assert!(ScopePattern::validate(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn as_str_round_trips() {
        for s in ["nation", "nation:a.gold", "nation:a.*"] {
            assert_eq!(ScopePattern::parse(s).as_str(), s);
        }
    }

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
