use lorealm_storage::models::{ConversationMessage, Param, RealmEvent, Snowflake};
use serde::Serialize;

use crate::services::chat::WidgetInstance;
use crate::services::commands::CommandEffect;

#[derive(Serialize)]
pub struct NextTurn {
    /// `None` when every agent is gated off at the active leaf.
    pub agent_id: Option<Snowflake>,
}

/// A bound tool projected for display
#[derive(Serialize)]
pub struct ActionSpec {
    pub tool_id: Snowflake,
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub event_channel: Option<String>,
}

/// A committed turn: the message it anchored to and what fanned out of it.
#[derive(Serialize)]
pub struct TurnResult {
    pub message: ConversationMessage,
    pub events: Vec<RealmEvent>,
}

/// What a client watching a live turn receives over SSE.
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RealmStreamEvent {
    AwaitingPlayer {
        agent_id: Snowflake,
    },
    TurnStarted {
        agent_id: Snowflake,
        name: String,
    },
    Delta {
        content: String,
    },
    Reasoning {
        content: String,
    },
    Widget {
        widget: WidgetInstance,
    },
    Effect {
        effect: CommandEffect,
    },
    Action {
        name: String,
        is_error: bool,
    },
    Committed {
        message: ConversationMessage,
        events: Vec<RealmEvent>,
    },
}
