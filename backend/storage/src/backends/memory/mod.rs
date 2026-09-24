pub mod blueprints;
pub mod characters;
pub mod connection;
pub mod conversations;
pub mod lorebook;
pub mod permissions;
pub mod plugins;
pub mod persona;
pub mod realms;
pub mod sessions;
pub mod state;
pub mod templates;
pub mod tokenizer;
pub mod tools;
pub mod users;
pub mod widgets;

use crate::StorageCore;
use crate::backends::memory::characters::StoredVersion;
use crate::backends::memory::lorebook::StoredLorebookVersion;
use crate::backends::memory::realms::{StoredRealm, StoredRealmVersion};
use crate::error::StorageError;
use crate::models::*;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug, Default)]
pub struct MemoryStorage {
    pub(crate) blueprints: RwLock<HashMap<Snowflake, Blueprint>>,
    pub(crate) users: RwLock<HashMap<Snowflake, User>>,
    pub(crate) sessions: RwLock<HashMap<String, Session>>,
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
    pub(crate) user_state: RwLock<HashMap<Snowflake, UserSettings>>,
    pub(crate) message_states: RwLock<HashMap<Snowflake, MessageState>>,
    pub(crate) personas: RwLock<HashMap<Snowflake, UserPersona>>,
    pub(crate) persona_overrides: RwLock<HashMap<Snowflake, PersonaOverride>>,
    pub(crate) secrets: RwLock<HashMap<Snowflake, Secret>>,
    pub(crate) connection_profiles: RwLock<HashMap<Snowflake, ConnectionProfile>>,
    pub(crate) user_tools: RwLock<HashMap<Snowflake, UserTool>>,
    pub(crate) user_widgets: RwLock<HashMap<Snowflake, UserWidget>>,
    pub(crate) user_plugins: RwLock<HashMap<Snowflake, Vec<UserPlugin>>>,
    pub(crate) plugin_data: RwLock<HashMap<(Snowflake, String, String), Vec<u8>>>,
    pub(crate) plugin_settings: RwLock<HashMap<(Snowflake, String), serde_json::Value>>,
    pub(crate) plugin_data_versions: RwLock<HashMap<(Snowflake, String), String>>,
    pub(crate) plugin_logs: RwLock<HashMap<(Snowflake, String), Vec<PluginLog>>>,
    pub(crate) tokenizers: RwLock<HashMap<Snowflake, Tokenizer>>,
    pub(crate) tokenizer_shares: RwLock<Vec<TokenizerShare>>,
    pub(crate) realms: RwLock<HashMap<Snowflake, StoredRealm>>,
    pub(crate) realm_versions: RwLock<HashMap<Snowflake, StoredRealmVersion>>,
    pub(crate) conversation_casts: RwLock<HashMap<Snowflake, RealmCast>>,
    pub(crate) realm_events: RwLock<HashMap<Snowflake, RealmEvent>>,
    pub(crate) agent_observations: RwLock<HashMap<Snowflake, AgentObservation>>,
    pub(crate) _realm_config: RwLock<HashMap<Snowflake, RealmConfig>>,
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

    /// Collect `root` and every descendant id
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
