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

    // llm only
    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,

    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
    #[serde(default)]
    pub speech_scope: Option<String>,
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub run_condition: Option<String>,
}

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
    #[serde(default)]
    pub args: serde_json::Value,
}

#[derive(Deserialize, Default)]
pub struct RunTurnRequest {
    #[serde(default)]
    pub target: Option<Snowflake>,
}

#[derive(Deserialize)]
pub struct EventsQuery {
    pub before: Option<Snowflake>,
}

#[derive(Deserialize)]
pub struct ObservationsQuery {
    pub before: Option<Snowflake>,
}
