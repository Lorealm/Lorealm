use std::{
    pin::Pin,
    sync::{Arc, Mutex as StdMutex},
};

use dashmap::DashMap;
use futures_util::Stream;
use lorealm_storage::models::{Conversation, ConversationMessage, Snowflake, StateDelta};
use serde::Serialize;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::ChatError,
    services::{
        chat::widgets::{InferEvent, ToolCallRecord, WidgetInstance, widget_marker},
        context::{PromptRole, TracedBlock},
        state::{CapturedState, StatePool, StateService},
    },
};

#[derive(Debug, Clone)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: String,
}

pub(super) enum Persist {
    Child(Snowflake),
    Sibling(Snowflake),
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
    /// Terminal: the persisted assistant message, carrying its real id so the
    /// client can reconcile the provisional bubble and fetch branch_info.
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
    pub fn text(&mut self, s: &str) {
        self.spoken.push_str(s);
    }
    pub fn reasoning(&mut self, s: &str) {
        self.reasoning.push_str(s);
    }
    pub fn tool_call(&mut self, r: ToolCallRecord) {
        self.tool_calls.push(r);
    }

    /// Inserts the placeholder marker into the spoken text and stores the
    /// widget. Returns the marker so the caller can forward it as a Delta.
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

/// A completed agent turn: what the agent said/did, plus the state writes its
/// tools produced (already write-scope-enforced at the tool layer).
pub struct AgentTurn {
    pub outcome: AgentOutcome,
    pub delta: StateDelta,
}

/// An agent turn with the inference stream still live, for callers that
/// forward events to a client while accumulating. Exhaust (or abandon) the
/// stream, then call [`Self::delta`] for what the turn's tools wrote.
pub struct OpenAgentTurn {
    pub stream: Pin<Box<dyn Stream<Item = Result<InferEvent, ChatError>> + Send>>,
    pub(super) state_pool: Arc<StdMutex<StatePool>>,
    pub(super) diff_base: StatePool,
}

impl OpenAgentTurn {
    pub fn delta(&self, state: &StateService) -> StateDelta {
        let pool = self.state_pool.lock().unwrap();
        state.diff(&self.diff_base, &pool)
    }
}

pub struct Agent {
    pub id: Snowflake,
    pub kind: AgentKindConcrete,
    pub persona_id: Snowflake, // see caveat below
}

pub enum AgentKindConcrete {
    Llm {
        character_id: Snowflake,
        char_version_id: Snowflake,
        template_id: Snowflake,
        profile_id: Snowflake,
    },
    Player {
        user_id: Snowflake,
    },
}

pub struct AgentView {
    pub realm_owner: Snowflake,
    pub tokenizer_id: Snowflake,
    pub thread: Vec<ConversationMessage>,
    pub state_slice: StatePool,
}
