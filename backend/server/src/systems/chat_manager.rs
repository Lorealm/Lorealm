use uuid::Uuid;

use crate::systems::types::Message;

pub struct ChatManager {
    pub owner_id: Uuid,
    pub chat_id: Uuid,
    pub messages: Vec<ChatSwipe>
}

/// Represents a horizontal swipe of messages
pub struct ChatSwipe {
    pub messages: Vec<Message>,
    pub selected_message: Option<Uuid>
}

impl ChatManager {
    pub fn new(owner_id: Uuid) -> ChatManager {
        ChatManager {
            owner_id,
            chat_id: Uuid::new_v4(),
            messages: Vec::new(),
        }
    }
    /// Adds a message to the end of the chat
    /// 
    /// Note: This creates a new swipe
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(ChatSwipe {
            messages: vec![message],
            selected_message: None
        });
    }

    /// Gets the messages in the chat
    /// 
    /// Will return the selected message within each swipe
    pub fn get_messages(&self) -> Vec<&Message> {
        let mut messages = Vec::new();
        for swipe in &self.messages {
            if let Some(message_id) = swipe.selected_message {
                for message in &swipe.messages {
                    if message.id == message_id {
                        messages.push(message);
                        break;
                    }
                }
            }
        }
        messages
    }

    pub fn get_chat_id(&self) -> Uuid {
        self.chat_id
    }

    pub fn remove_swipe(&mut self, index: usize) {
        self.messages.remove(index);
    }

    pub fn remove_message(&mut self, message_id: Uuid) {
        for swipe in &mut self.messages {
            swipe.messages.retain(|message| message.id != message_id);
        }
    }
}