use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use lorealm_encryption::LocalKeyManager;
use lorealm_storage::StorageManager;
use tokio::task::JoinHandle;

use crate::{
    config::{CharacterConfig, ConfigProvider, SessionConfig, TokenizerConfig, UserConfig},
    services::{
        character::CharacterService,
        chat::ChatService,
        commands::{CommandService, Generation},
        connection::ConnectionService,
        context::{ContextService, SourceRegistry, builtin::register_builtin_sources},
        conversation::ConversationService,
        lorebook::LorebookService,
        permission::service::PermissionService,
        persona::PersonaService,
        realm::RealmService,
        session::SessionService,
        snowflake::SnowflakeService,
        state::StateService,
        tokenizer::TokenizerService,
        tools::UserToolService,
        user_widgets::UserWidgetService,
        users::UserService,
        variables::{VariableRegistry, builtin::register_builtin_providers},
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
    chat: Arc<ChatService>,
    commands: Arc<CommandService>,
    persona: Arc<PersonaService>,
    connection: Arc<ConnectionService>,
    tools: Arc<UserToolService>,
    widgets: Arc<UserWidgetService>,
    tokenizers: Arc<TokenizerService>,
    realms: Arc<RealmService>,
}

impl ServiceManager {
    pub fn new(storage_manager: &StorageManager, config: Arc<ConfigProvider>) -> Self {
        // General services
        let storage = storage_manager.storage();
        let blob = storage_manager.blob();
        let snowflake = Arc::new(SnowflakeService::new(0)); // TODO: Change or derive from config

        let http_client = reqwest::Client::new();

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
            config.get::<CharacterConfig>(),
        ));

        let state = Arc::new(StateService::new(Arc::clone(&storage)));

        let sources = {
            let mut registry = SourceRegistry::new();
            register_builtin_sources(&mut registry, Arc::clone(&state));
            Arc::new(registry)
        };

        // The one definition of `{{scope:name}}`, shared by every weaver host.
        let variables = {
            let mut registry = VariableRegistry::new();
            register_builtin_providers(&mut registry);
            Arc::new(registry)
        };

        let context = Arc::new(ContextService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            Arc::clone(&sources),
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

        let conversation = Arc::new(ConversationService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let tools = Arc::new(UserToolService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
        ));

        let widgets = Arc::new(UserWidgetService::new(
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

        let tokenizers = Arc::new(TokenizerService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            Arc::clone(&blob),
            http_client.clone(),
            config.get::<TokenizerConfig>(),
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
            Arc::clone(&tokenizers),
            Arc::clone(&sources),
            Arc::clone(&variables),
        ));

        // The generate commands a tool body may call need an owned handle to
        // the service that runs them.
        chat.set_self(Arc::downgrade(&chat));

        let commands = Arc::new(CommandService::new(
            Arc::clone(&storage),
            Arc::clone(&conversation),
            Arc::clone(&state),
            Arc::clone(&tools),
            Arc::clone(&chat) as Arc<dyn Generation>,
        ));

        let realms = Arc::new(RealmService::new(
            Arc::clone(&storage),
            Arc::clone(&snowflake),
            Arc::clone(&chat),
            Arc::clone(&characters),
            Arc::clone(&context),
            Arc::clone(&connection),
            Arc::clone(&persona),
            Arc::clone(&state),
            Arc::clone(&conversation),
        ));

        let cleanable: Vec<Arc<dyn CleanableService>> = vec![
            Arc::clone(&characters) as Arc<dyn CleanableService>,
            Arc::clone(&lorebook) as Arc<dyn CleanableService>,
            Arc::clone(&tokenizers) as Arc<dyn CleanableService>,
            Arc::clone(&realms) as Arc<dyn CleanableService>,
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
            chat,
            commands,
            persona,
            connection,
            tools,
            widgets,
            tokenizers,
            realms,
        }
    }

    /// Spawns a background task that sweeps every cleanable service on `interval`.
    ///
    /// The first sweep runs after one `interval` has elapsed. Returns the
    /// [`JoinHandle`] so the caller can keep it alive or abort it on shutdown.
    pub fn start_cleanup_task(&self, interval: Duration) -> JoinHandle<()> {
        let cleanable = Arc::clone(&self.cleanable);

        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval);
            ticker.tick().await;

            tracing::info!(
                interval_secs = interval.as_secs(),
                services = cleanable.len(),
                "cleanup task started"
            );

            loop {
                ticker.tick().await;

                let started = std::time::Instant::now();
                let mut total = 0;

                for service in cleanable.iter() {
                    let cleaned = service.clean().await;
                    total += cleaned;

                    if cleaned > 0 {
                        tracing::debug!(
                            service = service.name(),
                            cleaned,
                            "swept cleanable service"
                        );
                    }
                }

                if total > 0 {
                    tracing::info!(
                        total,
                        elapsed_ms = started.elapsed().as_millis(),
                        "cleanup sweep complete"
                    );
                } else {
                    tracing::trace!(
                        elapsed_ms = started.elapsed().as_millis(),
                        "cleanup sweep idle"
                    );
                }
            }
        })
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

    pub fn commands(&self) -> &Arc<CommandService> {
        &self.commands
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

    pub fn widgets(&self) -> &Arc<UserWidgetService> {
        &self.widgets
    }

    pub fn tokenizers(&self) -> &Arc<TokenizerService> {
        &self.tokenizers
    }

    pub fn realm(&self) -> &Arc<RealmService> {
        &self.realms
    }
}

#[async_trait]
pub trait CleanableService: Send + Sync {
    /// The name of the service, used for logging
    fn name(&self) -> &'static str;
    /// Cleans the service.
    ///
    /// Returns amount of items cleaned.
    async fn clean(&self) -> usize;
}
