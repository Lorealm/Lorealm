pub mod characters;
pub mod conversations;
pub mod lorebook;
pub mod permissions;
pub mod sessions;
pub mod state;
pub mod templates;
pub mod users;

use crate::StorageCore;
use crate::backends::memory::characters::StoredVersion;
use crate::backends::memory::lorebook::StoredLorebookVersion;
use crate::error::StorageError;
use crate::models::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    pub(crate) users: RwLock<HashMap<Snowflake, User>>,
    pub(crate) sessions: RwLock<HashMap<String, Session>>, // keyed by token
    pub(crate) roles: RwLock<HashMap<Snowflake, Role>>,
    pub(crate) user_roles: RwLock<Vec<UserRole>>,
    pub(crate) characters: RwLock<HashMap<Snowflake, Character>>,
    pub(crate) character_versions: RwLock<HashMap<Snowflake, StoredVersion>>,
    pub(crate) templates: RwLock<HashMap<Snowflake, Template>>,
    pub(crate) lorebook_versions: RwLock<HashMap<Snowflake, StoredLorebookVersion>>,
    pub(crate) lorebooks: RwLock<HashMap<Snowflake, Lorebook>>,
    pub(crate) enabled_lorebooks: RwLock<HashMap<Snowflake, Vec<Snowflake>>>,
    pub(crate) conversations: RwLock<HashMap<Snowflake, Conversation>>,
    pub(crate) messages: RwLock<HashMap<Snowflake, ConversationMessage>>,
    pub(crate) user_state: RwLock<HashMap<Snowflake, Option<Snowflake>>>, // only one item in state for now
    pub(crate) message_states: RwLock<HashMap<Snowflake, MessageState>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn characters(&self) -> &RwLock<HashMap<Snowflake, Character>> {
        &self.characters
    }

    /// Count messages on the active path (leaf -> root) by walking `parent`.
    fn active_path_len(
        messages: &HashMap<Snowflake, ConversationMessage>,
        leaf: Option<Snowflake>,
    ) -> i64 {
        let mut count = 0i64;
        let mut current = leaf;
        while let Some(id) = current {
            match messages.get(&id) {
                Some(m) => {
                    count += 1;
                    current = m.parent;
                }
                None => break,
            }
        }
        count
    }

    /// Collect `root` and every descendant id (the subtree a cascade delete
    /// would remove).
    fn collect_subtree(
        messages: &HashMap<Snowflake, ConversationMessage>,
        root: Snowflake,
    ) -> Vec<Snowflake> {
        let mut found = vec![root];
        let mut stack = vec![root];
        while let Some(parent_id) = stack.pop() {
            for (id, m) in messages.iter() {
                if m.parent == Some(parent_id) {
                    found.push(*id);
                    stack.push(*id);
                }
            }
        }
        found
    }
}

#[async_trait]
impl StorageCore for MemoryStorage {
    async fn is_available(&self) -> bool {
        true
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        Ok(())
    }

    fn backend_type(&self) -> &str {
        "memory"
    }
}
