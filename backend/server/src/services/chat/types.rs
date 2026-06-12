use lorealm_storage::models::{Conversation, ConversationMessage, Snowflake};
use serde::Serialize;

use crate::services::{chat::widgets::WidgetInstance, context::{PromptRole, TracedBlock}, state::CapturedState};

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
    Reasoning { content: String },
    Widget { widget: WidgetInstance },
    /// Terminal: the persisted assistant message, carrying its real id so the
    /// client can reconcile the provisional bubble and fetch branch_info.
    Done {
        message: ConversationMessage,
    },
}
