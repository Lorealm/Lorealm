use lorealm_storage::models::{Param, Realm, RealmAgent, RealmEvent, Snowflake};
use serde::Serialize;

use crate::services::chat::WidgetInstance;

pub trait TurnPolicy: Send + Sync {
    fn next<'a>(&self, realm: &Realm, agents: &'a [RealmAgent]) -> Option<&'a RealmAgent>;
}

pub struct RoundRobin;

impl TurnPolicy for RoundRobin {
    fn next<'a>(&self, realm: &Realm, agents: &'a [RealmAgent]) -> Option<&'a RealmAgent> {
        if agents.is_empty() {
            return None;
        }
        agents.get((realm.tick % agents.len() as i64) as usize)
    }
}

/// A bound tool projected for display
#[derive(Serialize)]
pub struct ActionSpec {
    pub tool_id: Snowflake,
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub event_scope: Option<String>,
}

pub enum TickResult {
    /// An LLM agent acted and the tick committed.
    Committed { tick: i64, events: Vec<RealmEvent> },
    /// The policy selected a player; nothing committed. The client should
    /// prompt that user and call submit_player_turn (`/api/v1/realms/{realm_id}/turn`).
    AwaitingPlayer { agent_id: Snowflake },
}

/// What a client watching a live tick receives over SSE.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealmStreamEvent {
    /// The policy picked a player; nothing will stream. Prompt that user and
    /// call submit_player_turn.
    AwaitingPlayer { agent_id: Snowflake },
    /// An LLM agent's turn began.
    TurnStarted { agent_id: Snowflake, name: String },
    /// A chunk of the agent's spoken text.
    Delta { content: String },
    Reasoning { content: String },
    Widget { widget: WidgetInstance },
    /// A tool ran. Deliberately thin: inputs/results stay actor-private;
    /// the committed events carry whatever the realm may see.
    Action { name: String, is_error: bool },
    /// Terminal: the tick committed with these events.
    Committed { tick: i64, events: Vec<RealmEvent> },
}
