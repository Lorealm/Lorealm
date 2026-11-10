use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{RealmOrigin, Snowflake};

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
    /// The blueprint this realm was expanded from
    #[serde(default)]
    pub origin: Option<RealmOrigin>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealmDefinition {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// The cast. Array order is turn order — a template's loop produces its
    /// members in order, so there is nothing else to keep in sync.
    #[serde(default)]
    pub agents: Vec<RealmAgent>,
    /// Named openings; instantiation picks one (see [`Scenario`]).
    #[serde(default)]
    pub scenarios: Vec<Scenario>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// How the realm decides who acts next. `None` = [`Director::default`].
    #[serde(default)]
    pub director: Option<Director>,
    #[serde(default)]
    pub config: RealmConfig,
}

/// How a realm decides who acts next.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Director {
    /// Rotate through the cast in turn order, resuming after whoever acted
    /// last.
    RoundRobin {
        #[serde(default)]
        over: AgentFilter,
    },
    /// Pick in a random order.
    Random {
        #[serde(default)]
        over: AgentFilter,
    },
    /// A weaver expression naming who acts next
    Script { source: String },
}

impl Default for Director {
    fn default() -> Self {
        Self::RoundRobin {
            over: AgentFilter::All,
        }
    }
}

/// Which cast members a director draws from.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(tag = "match", rename_all = "snake_case")]
pub enum AgentFilter {
    #[default]
    All,
    Kind {
        kind: AgentKind,
    },
    Handles {
        handles: Vec<String>,
    },
}

impl AgentFilter {
    pub fn admits(&self, agent: &RealmAgent) -> bool {
        match self {
            Self::All => true,
            Self::Kind { kind } => &agent.kind == kind,
            Self::Handles { handles } => handles.contains(&agent.handle),
        }
    }
}

/// A named opening for a realm, analogous to a character greeting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    #[serde(default)]
    pub slug: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub opening: Option<String>,
}

/// A kind of owner-scoped resource a realm's cast can reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RealmResourceKind {
    Character,
    Lorebook,
    Template,
    Tool,
    Persona,
    ConnectionProfile,
    Widget,
}

impl RealmResourceKind {
    pub const ALL: [Self; 7] = [
        Self::Character,
        Self::Lorebook,
        Self::Template,
        Self::Tool,
        Self::Persona,
        Self::ConnectionProfile,
        Self::Widget,
    ];

    /// The name a WTN `Ref<...>` declaration writes.
    pub fn ref_name(&self) -> &'static str {
        match self {
            Self::Character => "Character",
            Self::Lorebook => "Lorebook",
            Self::Template => "Template",
            Self::Tool => "Tool",
            Self::Persona => "Persona",
            Self::ConnectionProfile => "ConnectionProfile",
            Self::Widget => "Widget",
        }
    }

    pub fn from_ref_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.ref_name() == name)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RealmResourceKind::Character => "character",
            RealmResourceKind::Lorebook => "lorebook",
            RealmResourceKind::Template => "template",
            RealmResourceKind::Tool => "tool",
            RealmResourceKind::Persona => "persona",
            RealmResourceKind::ConnectionProfile => "connection profile",
            RealmResourceKind::Widget => "widget",
        }
    }
}

/// One resource a realm references. Derived from the cast rather than
/// declared: see `RealmDefinition::resources`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RealmResourceRef {
    pub kind: RealmResourceKind,
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
    #[serde(default)]
    pub description: String,
    pub tags: Vec<String>,
    #[serde(default)]
    pub origin: Option<RealmOrigin>,
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
}

#[derive(Debug, Clone)]
pub struct ResolvedRealmConfig {
    pub speech_format: String,
    pub action_format: String,
    pub tool_result_format: String,
}

impl Default for ResolvedRealmConfig {
    fn default() -> Self {
        Self {
            speech_format: "{name}: {content}".into(),
            action_format: "[{name} used {tool}]".into(),
            tool_result_format: "[{tool} result: {output}]".into(),
        }
    }
}

impl ResolvedRealmConfig {
    /// Layer a realm's overrides over the built-in defaults.
    pub fn from_config(config: &RealmConfig) -> Self {
        let mut cfg = Self::default();
        cfg._apply_config(config);
        cfg
    }

    fn _apply_config(&mut self, o: &RealmConfig) {
        if let Some(v) = &o.speech_format {
            self.speech_format = v.clone();
        }
        if let Some(v) = &o.action_format {
            self.action_format = v.clone();
        }
        if let Some(v) = &o.tool_result_format {
            self.tool_result_format = v.clone();
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

    /// An actor's private note (string-formatted tool result) after a tool runs.
    /// `{tool}`, `{output}`.
    pub fn tool_result(&self, tool: &str, output: &str) -> String {
        self.tool_result_format
            .replace("{tool}", tool)
            .replace("{output}", output)
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
    pub handle: String,
    pub name: String,
    pub kind: AgentKind,

    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,
    pub persona_id: Option<Snowflake>,

    pub user_id: Option<Snowflake>,

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
    /// Lorebooks feeding this agent's context assembly. Empty = whichever
    /// lorebooks the owner has active when the turn runs.
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    /// Imported static display widgets shown when this agent is the viewer.
    #[serde(default)]
    pub widgets: Vec<Snowflake>,
    /// Inline weaver source run on this agent's turn.
    #[serde(default)]
    pub script: Option<String>,
    /// Weaver expression gating whether the agent runs on its turn.
    #[serde(default)]
    pub run_condition: Option<String>,
}

impl RealmAgent {
    /// The implicit grant every agent holds on its own subtree:
    /// `agents:<handle>.*`.
    pub fn own_scope(&self) -> ScopePattern {
        ScopePattern {
            scope: AGENTS_SCOPE.to_string(),
            name: NamePattern::Under(format!("{}.", self.handle)),
        }
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

/// The cast a conversation plays with.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmCast {
    /// The realm version this playthrough was instantiated from.
    #[serde(default)]
    pub version_id: Option<Snowflake>,
    pub agents: Vec<RealmAgent>,
    #[serde(default)]
    pub director: Option<Director>,
    #[serde(default)]
    pub scenario: Option<Scenario>,
    #[serde(default)]
    pub config: RealmConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    Spoke,
    Acted,
    System,
}

/// Something that happened on a turn besides the turn's own text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEvent {
    pub channel: String,
    pub kind: EventKind,
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
    /// Whether this grant covers the variable `scope:name`. A bare `*`
    /// covers everything.
    pub fn matches_var(&self, scope: &str, name: &str) -> bool {
        self.scope == "*"
            || self.scope == scope
                && match &self.name {
                    NamePattern::Any => true,
                    NamePattern::Exact(n) => name == n,
                    NamePattern::Under(prefix) => name.starts_with(prefix.as_str()),
                }
    }

    /// Whether this grant observes events on `channel`
    pub fn matches_channel(&self, channel: &str) -> bool {
        self.scope == "*" || self.scope == channel
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
        if s == "*" {
            return Ok(());
        }
        if !is_identifier(scope) {
            return Err(format!(
                "grant scope '{scope}' must be `*` or a bare identifier ([A-Za-z_][A-Za-z0-9_]*)"
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

        let cfg = ResolvedRealmConfig::from_config(&config);

        assert_eq!(cfg.speech("A", "hi"), "<A> hi"); // override
        assert_eq!(cfg.action("A", "cast"), "[A used cast]"); // default
        assert_eq!(cfg.tool_result("cast", "ok"), "[cast result: ok]"); // default
    }
}
