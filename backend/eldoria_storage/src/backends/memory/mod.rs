pub mod permissions;
pub mod sessions;
pub mod users;

use crate::StorageCore;
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
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self::default()
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
