pub mod characters;
pub mod lorebook;
pub mod permissions;
pub mod sessions;
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
    pub(crate) lorebook_sessions: RwLock<HashMap<Snowflake, LorebookSession>>,
    pub(crate) lorebooks: RwLock<HashMap<Snowflake, Lorebook>>,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn characters(&self) -> &RwLock<HashMap<Snowflake, Character>> {
        &self.characters
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
