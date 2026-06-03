use eldoria_storage::models::{Conversation, ConversationMessage};
use serde::Serialize;

#[derive(Serialize)]
pub struct FullConversation {
    pub metadata: Conversation,
    pub messages: Vec<ConversationMessage>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChatStreamEvent {
    /// A chunk of generated text. Client appends to a provisional bubble.
    Delta { content: String },
    /// Terminal: the persisted assistant message, carrying its real id so the
    /// client can reconcile the provisional bubble and fetch branch_info.
    Done { message: ConversationMessage },
}
