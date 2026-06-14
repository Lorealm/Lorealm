use std::sync::Arc;

use async_trait::async_trait;
use lorealm_encryption::LocalKeyManager;
use lorealm_storage::StorageManager;

use crate::{
    config::{ConfigProvider, SessionConfig, UserConfig},
    services::{
        character::CharacterService, chat::ChatService, connection::ConnectionService,
        context::ContextService, conversation::ConversationService, lorebook::LorebookService,
        permission::service::PermissionService, persona::PersonaService, session::SessionService,
        snowflake::SnowflakeService, state::StateService, tools::UserToolService,
        users::UserService,
    },
};

pub struct ServiceManager {
    /// A list of services which can be cleaned via a lazy routine sweep
    cleanable: Arc<Vec<Arc<dyn CleanableService>>>,
    sessions: Arc<SessionService>,
    users: Arc<UserService>,
    permissions: Arc<PermissionService>,
    characters: Arc<CharacterService>,
    context: Arc<ContextService>,
    lorebook: Arc<LorebookService>,
    state: Arc<StateService>,
    conversation: Arc<ConversationService>,
    chat: Arc<ChatService>,
    persona: Arc<PersonaService>,
    connection: Arc<ConnectionService>,
    tools: Arc<UserToolService>,
}

impl ServiceManager {
    pub fn new(storage_manager: &StorageManager, config: Arc<ConfigProvider>) -> Self {
        // General services
        let storage = storage_manager.storage();
        let blob = storage_manager.blob();
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
            Arc::clone(&blob),
            Arc::clone(&snowflake),
        ));

        let context = Arc::new(ContextService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let lorebook = Arc::new(LorebookService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let persona = Arc::new(PersonaService::new(
            Arc::clone(&storage),
            Arc::clone(&blob),
            Arc::clone(&snowflake),
        ));

        let state = Arc::new(StateService::new(Arc::clone(&storage)));

        let conversation = Arc::new(ConversationService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let tools = Arc::new(UserToolService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let connection = Arc::new(ConnectionService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            Arc::new(
                LocalKeyManager::from_env_with_history("LOREALM_MASTER_KEY")
                    .expect("Should be able to create key manager"),
            ),
        ));

        let chat = Arc::new(ChatService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            Arc::clone(&conversation),
            Arc::clone(&characters),
            Arc::clone(&state),
            Arc::clone(&persona),
            Arc::clone(&connection),
            Arc::clone(&tools),
        ));

        let cleanable: Vec<Arc<dyn CleanableService>> = vec![
            Arc::clone(&characters) as Arc<dyn CleanableService>,
            Arc::clone(&lorebook) as Arc<dyn CleanableService>,
        ];

        Self {
            cleanable: Arc::new(cleanable),
            sessions,
            users,
            permissions,
            characters,
            context,
            lorebook,
            state,
            conversation,
            chat,
            persona,
            connection,
            tools,
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

    pub fn lorebooks(&self) -> &Arc<LorebookService> {
        &self.lorebook
    }

    pub fn chat(&self) -> &Arc<ChatService> {
        &self.chat
    }

    pub fn state(&self) -> &Arc<StateService> {
        &self.state
    }

    pub fn persona(&self) -> &Arc<PersonaService> {
        &self.persona
    }

    pub fn connections(&self) -> &Arc<ConnectionService> {
        &self.connection
    }

    pub fn tools(&self) -> &Arc<UserToolService> {
        &self.tools
    }
}

#[async_trait]
pub trait CleanableService: Send + Sync {
    /// Cleans the service.
    ///
    /// Returns amount of items cleaned.
    async fn clean(&self) -> usize;
}
