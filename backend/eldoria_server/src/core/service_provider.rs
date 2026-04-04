use std::sync::Arc;

use eldoria_storage::StorageManager;

use crate::{
    config::{ConfigProvider, SessionConfig, UserConfig},
    services::{session::SessionService, snowflake::SnowflakeService, users::UserService},
};

pub struct ServiceProvider {
    sessions: Arc<SessionService>,
    users: Arc<UserService>,
}

impl ServiceProvider {
    pub fn new(storage_manager: &StorageManager, config: Arc<ConfigProvider>) -> Self {
        // General services
        let storage = storage_manager.storage();
        let snowflake = Arc::new(SnowflakeService::new(0)); // TODO: Change or derive from config

        // Session Service
        let sessions = Arc::new(SessionService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            config.get::<SessionConfig>(),
        ));

        let users = Arc::new(UserService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            config.get::<UserConfig>(),
            Arc::clone(&sessions),
        ));

        Self { sessions, users }
    }

    pub fn sessions(&self) -> &Arc<SessionService> {
        &self.sessions
    }

    pub fn users(&self) -> &Arc<UserService> {
        &self.users
    }
}
