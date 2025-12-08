use chrono::Utc;
use uuid::Uuid;

use crate::{Conversation, Message, MessageRole, User};

pub struct ConversationBuilder {
    conversation: Conversation,
}

impl Default for ConversationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ConversationBuilder {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            conversation: Conversation {
                id: Uuid::new_v4(),
                user_id: Uuid::new_v4(),
                created_at: now,
                updated_at: now,
                context_type: "empty".to_string(),
                context_id: None,
                context_version: None,
                context_metadata: None,
                sync_policy: "default".to_string(),
            },
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.conversation.id = id;
        self
    }

    pub fn user_id(mut self, user_id: Uuid) -> Self {
        self.conversation.user_id = user_id;
        self
    }

    pub fn context_type(mut self, context_type: impl Into<String>) -> Self {
        self.conversation.context_type = context_type.into();
        self
    }

    pub fn context_id(mut self, context_id: Uuid) -> Self {
        self.conversation.context_id = Some(context_id);
        self
    }

    pub fn context_version(mut self, version: u64) -> Self {
        self.conversation.context_version = Some(version);
        self
    }

    pub fn sync_policy(mut self, policy: impl Into<String>) -> Self {
        self.conversation.sync_policy = policy.into();
        self
    }

    pub fn build(self) -> Conversation {
        self.conversation
    }
}

pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    pub fn new(conversation_id: Uuid, parent: Option<Uuid>) -> Self {
        let now = Utc::now();
        Self {
            message: Message {
                id: Uuid::new_v4(),
                parent_message_id: parent,
                conversation_id,
                role: MessageRole::User,
                content: "Test message".to_string(),
                created_at: now,
                updated_at: now,
            },
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.message.id = id;
        self
    }

    pub fn role(mut self, role: MessageRole) -> Self {
        self.message.role = role;
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.message.content = content.into();
        self
    }

    pub fn build(self) -> Message {
        self.message
    }
}

pub struct UserBuilder {
    user: User,
}

impl Default for UserBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl UserBuilder {
    pub fn new() -> Self {
        Self {
            user: User {
                id: Uuid::new_v4(),
                permissions: 0,
                email: Some("test@example.com".to_string()),
                name: "Test User".to_string(),
                password: Some("VeryArticulatePassword".to_string()),
                created_at: Utc::now(),
            },
        }
    }

    pub fn id(mut self, id: Uuid) -> Self {
        self.user.id = id;
        self
    }

    pub fn email(mut self, email: impl Into<String>) -> Self {
        self.user.email = Some(email.into());
        self
    }

    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.user.name = name.into();
        self
    }

    pub fn build(self) -> User {
        self.user
    }
}
