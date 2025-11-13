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

impl ContextEntry {
    pub fn new(content: &str, role: &str, source: &str) -> Self {
        Self { message: 
            Message { 
                role: role.to_string(),
                content: content.to_string()
            }, 
            source: source.to_string()
        }
    }
}