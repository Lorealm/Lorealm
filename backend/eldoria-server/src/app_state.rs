use std::sync::Arc;

use secrecy::SecretString;
use storage_system::{Storage, StorageConfig, StorageManager};
use tokio::sync::RwLock;
use crate::models::StorageService;

use crate::{
    models::{CharacterService, SessionService, TemplateService, UserService},
    services::{
        authentication::SessionConfig,
        context::SourceRegistry,
    },
};

#[derive(Debug, Clone)]
pub struct StorageProvider {
    storage_manager: StorageManager,
    session_config: SessionConfig,
}

impl StorageProvider {
    pub fn new(manager: StorageManager) -> Self {
        Self {
            storage_manager: manager,
            session_config: SessionConfig::default(),
        }
    }

    pub fn character_service(&self) -> Arc<CharacterService<dyn Storage>> {
        let storage = self.storage_manager.storage();
        Arc::new(CharacterService::new(storage))
    }

    pub fn user_service(&self) -> Arc<UserService<dyn Storage>> {
        let storage = self.storage_manager.storage();
        Arc::new(UserService::new(storage))
    }

    pub fn session_service(&self) -> Arc<SessionService<dyn Storage>> {
        let storage = self.storage_manager.storage();
        Arc::new(SessionService::new_with_config(
            storage,
            self.session_config.clone(),
        ))
    }

    pub fn template_service(&self) -> Arc<TemplateService<dyn Storage>> {
        let storage = self.storage_manager.storage();
        Arc::new(TemplateService::new(storage))
    }

    pub fn session_config(&self) -> SessionConfig {
        self.session_config.clone()
    }
}

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub max_card_upload_size_mb: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            max_card_upload_size_mb: 20,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    storage: Arc<RwLock<StorageProvider>>,
    server_secret: SecretString,
    config: ServerConfig,
    
    /// Registry of available context sources (runtime only, not persisted)
    source_registry: SourceRegistry,
}

// Manual Debug that hides the secret
impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("storage", &self.storage)
            .field("server_secret", &"[REDACTED]")
            .field("config", &self.config)
            .field("source_registry", &self.source_registry)
            .finish()
    }
}

impl AppState {
    pub async fn new(storage_config: StorageConfig) -> anyhow::Result<Self> {
        let manager = StorageManager::new(storage_config).await?;
        let secret = std::env::var("ELDORIA_SERVER_SECRET")?;

        Ok(Self {
            storage: Arc::new(RwLock::new(StorageProvider::new(manager))),
            server_secret: SecretString::new(secret.into_boxed_str()),
            config: ServerConfig::default(),
            source_registry: SourceRegistry::new(),
        })
    }

    pub fn storage(&self) -> Arc<RwLock<StorageProvider>> {
        Arc::clone(&self.storage)
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Get a reference to the source registry
    pub fn source_registry(&self) -> &SourceRegistry {
        &self.source_registry
    }

    /// Get a mutable reference to register custom sources
    pub fn source_registry_mut(&mut self) -> &mut SourceRegistry {
        &mut self.source_registry
    }

    // Return a reference to avoid copies
    pub(crate) fn server_secret(&self) -> &SecretString {
        &self.server_secret
    }
}