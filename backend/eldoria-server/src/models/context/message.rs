use uuid::Uuid;

/// This is a 1:1 clone of `storage_system::MessageRole`
/// 
/// Cloned to keep `storage_system` seperated and better scoped
pub enum MessageRole {
    User,
    Assistant,
    System
}

pub struct Conversation {
    pub id: Uuid,
    pub swipes: Vec<Swipe>
}

/// A container for every message.
pub struct Swipe {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub messages: Vec<Message>,
}

pub struct Message {
    pub id: Uuid,
    pub idx: u16,
    pub content: String,
    pub role: MessageRole
}