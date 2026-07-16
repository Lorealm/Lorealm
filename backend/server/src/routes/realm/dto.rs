use lorealm_storage::models::{AgentGrant, AgentKind, AgentToolBinding, Snowflake};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateRealmRequest {
    pub name: String,
}

/// Partial preset edit; omitted fields keep their value.
#[derive(Deserialize)]
pub struct UpdateRealmRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateAgentRequest {
    pub name: String,
    pub kind: AgentKind,

    // llm only — template/profile default from the caller's actives if omitted
    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,

    /// What the agent can know: scoped access to state namespaces.
    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    /// The agent's action space: which of the owner's tools it may call,
    /// and what scope each tool's Acted event carries. Empty = speak-only.
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
    /// The scope this agent's speech carries — who hears it. Omitted = the
    /// broadcast `main` channel.
    #[serde(default)]
    pub speech_scope: Option<String>,
    /// Which imported lorebooks feed this agent's context assembly.
    /// Empty = no lorebook context.
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    /// Script agents only: the inline weaver source their turn runs.
    #[serde(default)]
    pub script: Option<String>,
    /// Optional weaver expression gating whether the agent runs on its turn.
    /// Omitted = always runs. Any agent kind may carry one.
    #[serde(default)]
    pub run_condition: Option<String>,
}

/// A player turn: say something, perform one bound action, or both.
#[derive(Deserialize)]
pub struct PlayerTurnRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub action: Option<PlayerAction>,
}

#[derive(Deserialize)]
pub struct PlayerAction {
    pub tool_id: Snowflake,
    /// Arguments matching the tool's params, keyed by name.
    #[serde(default)]
    pub args: serde_json::Value,
}

/// Regenerate a specific turn: the new message becomes a sibling of
/// `target` (same agent, alternate world-line). Omitted = advance the path.
#[derive(Deserialize, Default)]
pub struct RunTurnRequest {
    #[serde(default)]
    pub target: Option<Snowflake>,
}

/// Backward paging for the playthrough timeline: return events strictly
/// before this event id.
#[derive(Deserialize)]
pub struct EventsQuery {
    pub before: Option<Snowflake>,
}

/// Backward paging for an agent's observation feed: return observations
/// strictly before this observation id.
#[derive(Deserialize)]
pub struct ObservationsQuery {
    pub before: Option<Snowflake>,
}
