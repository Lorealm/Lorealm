use std::{
    collections::HashMap,
    pin::Pin,
    sync::{Arc, Mutex as StdMutex},
};

use dashmap::DashMap;
use futures_util::Stream;
use lorealm_storage::models::{
    ActivationState, Conversation, ConversationMessage, Snowflake, StateDelta,
};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::ChatError,
    services::{
        chat::widgets::{InferEvent, ToolCallRecord, WidgetInstance, widget_marker},
        commands::CommandEffect,
        context_template::{PromptRole, TracedBlock},
        state::{CapturedState, StatePool, StateService},
    },
    util::context_eval::UsageReport,
};

#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

pub(super) enum Persist {
    Child(Snowflake),
    Sibling(Snowflake),
    /// Extend an existing assistant message in place
    Append(ConversationMessage),
}

pub(super) struct TurnPlan {
    pub assembly_leaf: Option<Snowflake>,
    pub created_user: Option<ConversationMessage>,
    pub persist: Persist,
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

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    UserMessage {
        message: ConversationMessage,
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
    /// A system message a tool wrote with `$[sys]`, persisted after the reply
    /// it accompanied and therefore arriving after [`ChatStreamEvent::Done`].
    SystemMessage {
        message: ConversationMessage,
    },
    /// The persisted assistant message.
    Done {
        message: ConversationMessage,
    },
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

pub struct OpenAgentTurn {
    pub stream: Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>,
    /// The assembled context trace, for stamping into the turn's metadata.
    pub trace: Vec<TracedBlock>,
    pub usage: UsageReport,
    pub available_tools: Vec<String>,
    pub(super) state_pool: Arc<StdMutex<StatePool>>,
    pub(super) diff_base: StatePool,
    /// Full activation snapshot captured by assembly, keyed by lorebook id.
    pub activation: HashMap<Snowflake, ActivationState>,
    /// Whether assembly captured weaver state. When true the turn persists a
    /// MessageState row even if the pool delta is empty.
    pub had_assembly: bool,
}

impl OpenAgentTurn {
    pub fn delta(&self, state: &StateService) -> StateDelta {
        let pool = self.state_pool.lock().unwrap();
        state.diff(&self.diff_base, &pool)
    }
}
