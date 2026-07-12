use lorealm_storage::models::{AgentAccess, AgentKind, RealmEvent, Snowflake};
use serde::{Deserialize, Serialize};

use crate::services::realm::TickResult;

#[derive(Deserialize)]
pub struct CreateRealmRequest {
    pub name: String,
    pub tokenizer_id: Option<Snowflake>,
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

    #[serde(default)]
    pub grants: Vec<GrantDto>,
    /// The agent's action space: which of the owner's tools it may call,
    /// and what scope each tool's Acted event carries. Empty = speak-only.
    #[serde(default)]
    pub tools: Vec<ToolBindingDto>,
}

#[derive(Deserialize)]
pub struct ToolBindingDto {
    pub tool_id: Snowflake,
    /// Scope of the Acted event when this tool runs; omitted = private
    /// (only the actor observes its own action).
    #[serde(default)]
    pub event_scope: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct GrantDto {
    pub scope: String,
    pub access: AgentAccess,
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

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum TickResponse {
    Committed { tick: i64, events: Vec<RealmEvent> },
    AwaitingPlayer { agent_id: Snowflake },
}

impl From<TickResult> for TickResponse {
    fn from(value: TickResult) -> Self {
        match value {
            TickResult::Committed { tick, events } => TickResponse::Committed { tick, events },
            TickResult::AwaitingPlayer { agent_id } => TickResponse::AwaitingPlayer { agent_id },
        }
    }
}