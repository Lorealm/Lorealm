use std::collections::HashMap;

use lorealm_storage::models::{AgentGrant, AgentKind, AgentToolBinding, Director, Snowflake};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateRealmRequest {
    pub name: String,
}

/// A WTN document and the inputs to expand it with.
#[derive(Deserialize)]
pub struct ImportRealmRequest {
    pub source: String,
    /// Values for the document's `#inputs`.
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
pub struct UpdateRealmRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
    /// Outer `None` leaves the director alone; inner `None` resets it to the
    /// default rotation.
    #[serde(default, deserialize_with = "crate::util::double_option")]
    pub director: Option<Option<Director>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CreateAgentRequest {
    pub name: String,
    /// Weaver identifier, unique across the cast. See [`RealmAgent::handle`].
    ///
    /// [`RealmAgent::handle`]: lorealm_storage::models::RealmAgent::handle
    pub handle: String,
    pub kind: AgentKind,

    // llm only
    pub character_id: Option<Snowflake>,
    pub char_version_id: Option<Snowflake>,
    pub template_id: Option<Snowflake>,
    pub profile_id: Option<Snowflake>,

    pub persona_id: Option<Snowflake>,

    /// Seeded as `agents:<handle>.<key>` at instantiation.
    #[serde(default)]
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub grants: Vec<AgentGrant>,
    #[serde(default)]
    pub tools: Vec<AgentToolBinding>,
    #[serde(default)]
    pub speech_channel: Option<String>,
    #[serde(default)]
    pub lorebooks: Vec<Snowflake>,
    /// Static display widgets shown when this agent is the viewer.
    #[serde(default)]
    pub widgets: Vec<Snowflake>,
    #[serde(default)]
    pub script: Option<String>,
    #[serde(default)]
    pub run_condition: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateScenarioRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub opening: Option<String>,
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

#[derive(Deserialize)]
pub struct AgentStateQuery {
    /// Reconstruct state up to this message
    pub at: Option<Snowflake>,
}
