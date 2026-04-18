use std::sync::Arc;

use eldoria_storage::StorageManager;

use crate::{
    config::{ConfigProvider, SessionConfig, UserConfig},
    services::{
        character::CharacterService, context::ContextService,
        permission::service::PermissionService, session::SessionService,
        snowflake::SnowflakeService, users::UserService,
    },
};

pub struct ServiceProvider {
    sessions: Arc<SessionService>,
    users: Arc<UserService>,
    permissions: Arc<PermissionService>,
    characters: Arc<CharacterService>,
    context: Arc<ContextService>,
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

        let permissions = Arc::new(PermissionService::new(
            Arc::clone(&storage),
            Arc::clone(&users),
        ));

        let characters = Arc::new(CharacterService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let context = Arc::new(ContextService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        Self {
            sessions,
            users,
            permissions,
            characters,
            context,
        }
    }

    pub fn sessions(&self) -> &Arc<SessionService> {
        &self.sessions
    }

    pub fn users(&self) -> &Arc<UserService> {
        &self.users
    }

    pub fn permissions(&self) -> &Arc<PermissionService> {
        &self.permissions
    }

    pub fn characters(&self) -> &Arc<CharacterService> {
        &self.characters
    }
    
    pub fn context(&self) -> &Arc<ContextService> {
        &self.context
    }
}
