use lorealm_storage::models::{Param, Realm, RealmAgent, RealmEvent, Snowflake};
use serde::Serialize;

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
