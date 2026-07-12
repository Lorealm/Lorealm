use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

// copy-pasted from ContextWeaver 0.2.0
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActivationState {
    /// Entry ID -> turn number when it was last activated.
    pub last_activated: HashMap<String, usize>,
    /// Entry ID -> remaining sticky turns.
    pub sticky_remaining: HashMap<String, usize>,
    /// The current turn number
    pub current_turn: usize,
}

/// One assistant message's contribution to weaver state, as stored.
/// `activation` is a full snapshot per active lorebook; `persistent` is a
/// delta against the previous turn over the shared state pool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageState {
    pub message_id: Snowflake,
    /// Full activation snapshot, keyed by lorebook id.
    pub activation: HashMap<Snowflake, ActivationState>,
    /// Shared-pool delta
    pub persistent: StateDelta,
}

/// Per-turn change to the `state:` namespace.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateDelta {
    /// namespace → (name → new or changed value)
    pub writes: HashMap<String, HashMap<String, serde_json::Value>>,
    /// namespace → names removed this turn
    pub deletes: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct UserSettings {
    pub active_template_id: Option<Snowflake>,
    pub active_persona_id: Option<Snowflake>,
    pub active_connection_profile_id: Option<Snowflake>,
    pub active_tokenizer_id: Option<Snowflake>,
}

// Postgres-only
#[derive(sqlx::FromRow)]
pub(crate) struct MessageStateRow {
    message_id: Snowflake,
    activation: sqlx::types::Json<HashMap<Snowflake, ActivationState>>,
    persistent_delta: sqlx::types::Json<StateDelta>,
}

impl From<MessageStateRow> for MessageState {
    fn from(r: MessageStateRow) -> Self {
        MessageState {
            message_id: r.message_id,
            activation: r.activation.0,
            persistent: r.persistent_delta.0,
        }
    }
}
