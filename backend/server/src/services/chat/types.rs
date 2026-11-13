use std::sync::Arc;

use dashmap::DashMap;
use lorealm_storage::models::{Conversation, ConversationMessage, Param, Snowflake};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::{
    services::{
        chat::{
            AttachmentSpec,
            widgets::{ToolCallRecord, WidgetInstance, widget_marker},
        },
        commands::CommandEffect,
        context_template::{PromptRole, TracedBlock},
        state::CapturedState,
    },
    util::context_eval::UsageReport,
};

#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

pub(super) struct Prepared {
    pub prompt: Vec<PromptMessage>,
    pub trace: Vec<TracedBlock>,
    pub captured: Option<CapturedState>,
    pub usage: UsageReport,
}

#[derive(Serialize)]
pub struct FullConversation {
    pub metadata: Conversation,
    pub messages: Vec<ConversationMessage>,
}

/// One request to advance a conversation.
#[derive(Debug, Default, Deserialize)]
pub struct TurnRequest {
    /// What the player says.
    #[serde(default)]
    pub content: Option<String>,
    /// A bound tool the player triggers.
    #[serde(default)]
    pub action: Option<PlayerAction>,
    #[serde(default)]
    pub swipe_from: Option<Snowflake>,
    #[serde(default, rename = "continue")]
    pub continued: bool,
}

#[derive(Debug, Deserialize)]
pub struct PlayerAction {
    pub tool_id: Snowflake,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// What a client watching a turn receives over SSE.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    /// A cast member began acting.
    TurnStarted {
        handle: String,
        name: String,
    },
    /// A chunk of generated text. Client appends to a provisional bubble.
    Delta {
        content: String,
    },
    Reasoning {
        content: String,
    },
    Widget {
        widget: WidgetInstance,
    },
    /// A tool asked the client to do something
    Effect {
        effect: CommandEffect,
    },
    /// A message was persisted through any means (agent turn, contiuation, `$[sys]`, etc.)
    Message {
        message: ConversationMessage,
    },
    /// The request is finished. `next` is whose turn it is now, if anyone's.
    Done {
        next: Option<String>,
    },
}

#[derive(Serialize)]
pub struct ActionSpec {
    pub tool_id: Snowflake,
    pub name: String,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub event_channel: Option<String>,
}

#[derive(Serialize)]
pub struct NextTurn {
    /// `None` when nobody in the cast can act at the active leaf.
    pub handle: Option<String>,
}

pub struct ActiveWidgets {
    pub prompts: Vec<String>,
    pub renders: Vec<String>,
    pub tools: Vec<Snowflake>,
    pub attachments: Vec<AttachmentSpec>,
}

pub struct CancelGuard {
    pub map: Arc<DashMap<Snowflake, CancellationToken>>,
    pub key: Snowflake,
}
impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.map.remove(&self.key);
    }
}

#[derive(Default)]
pub struct TurnAccumulator {
    spoken: String,
    reasoning: String,
    widgets: Vec<WidgetInstance>,
    tool_calls: Vec<ToolCallRecord>,
}

impl TurnAccumulator {
    /// Resume an interrupted turn.
    pub fn resuming(prior: &ConversationMessage) -> Self {
        fn field<T: serde::de::DeserializeOwned + Default>(
            metadata: &serde_json::Value,
            key: &str,
        ) -> T {
            metadata
                .get(key)
                .cloned()
                .and_then(|v| serde_json::from_value(v).ok())
                .unwrap_or_default()
        }
        Self {
            spoken: prior.content.clone(),
            reasoning: field(&prior.metadata, "reasoning"),
            widgets: field(&prior.metadata, "widgets"),
            tool_calls: field(&prior.metadata, "tool_calls"),
        }
    }

    pub fn text(&mut self, s: &str) {
        self.spoken.push_str(s);
    }
    pub fn reasoning(&mut self, s: &str) {
        self.reasoning.push_str(s);
    }
    pub fn tool_call(&mut self, r: ToolCallRecord) {
        self.tool_calls.push(r);
    }

    pub fn widget(&mut self, w: WidgetInstance) -> String {
        let marker = widget_marker(self.widgets.len());
        self.spoken.push_str(&marker);
        self.widgets.push(w);
        marker
    }

    pub fn finish(self) -> AgentOutcome {
        AgentOutcome {
            spoken: self.spoken,
            reasoning: self.reasoning,
            widgets: self.widgets,
            tool_calls: self.tool_calls,
        }
    }
}

pub struct AgentOutcome {
    pub spoken: String,
    pub reasoning: String,
    pub widgets: Vec<WidgetInstance>,
    pub tool_calls: Vec<ToolCallRecord>,
}
