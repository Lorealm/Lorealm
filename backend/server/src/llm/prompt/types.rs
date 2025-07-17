use llm_providers::types::Message;
use uuid::Uuid;

pub struct Prompt {
    id: Uuid,
    context: Vec<ContextEntry>
}

pub struct ContextEntry {
    pub message: Message,
    pub source: String
}