use std::sync::{Arc, OnceLock};

use lorealm_storage::{StorageConfig, StorageManager};
use secrecy::SecretString;

use crate::{
    config::ConfigProvider,
    core::{ServiceManager, secrets::Secrets},
};

#[derive(Clone)]
pub struct AppState {
    config_provider: Arc<ConfigProvider>,
    service_manager: Arc<OnceLock<Arc<ServiceManager>>>,
    secret: Arc<OnceLock<SecretString>>,
}

impl AppState {
    pub(crate) async fn new() -> anyhow::Result<Self> {
        let path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.yaml".into())
            .into();

        let config = Arc::new(ConfigProvider::load(path)?);

        Ok(Self {
            config_provider: config,
            service_manager: Arc::new(OnceLock::new()),
            secret: Arc::new(OnceLock::new()),
        })
    }

    /// Build the storage backend + services and install them into the shared
    /// cell. Call once the one-time setup flow completes.
    pub(crate) async fn init_storage(&self, storage_config: StorageConfig) -> anyhow::Result<()> {
        let secrets = Secrets::load(&storage_config.data_directory)?;
        let manager = StorageManager::new(storage_config).await?;
        let service_manager = Arc::new(ServiceManager::new(
            &manager,
            Arc::clone(&self.config_provider),
            secrets.key_manager,
        ));

        self.secret
            .set(secrets.session_secret)
            .map_err(|_| anyhow::anyhow!("secrets have already been loaded"))?;

        self.service_manager
            .set(service_manager)
            .map_err(|_| anyhow::anyhow!("storage has already been initialized"))?;

        Ok(())
    }

    /// `Some` once setup is done, `None` while awaiting configuration.
    pub(crate) fn services(&self) -> &Arc<ServiceManager> {
        self.service_manager
            .get()
            .expect("service manager should be available. has it been initialized?")
    }

    pub(crate) fn config(&self) -> &Arc<ConfigProvider> {
        &self.config_provider
    }

    pub(crate) fn secret(&self) -> SecretString {
        self.secret
            .get()
            .expect("secrets should be loaded. has storage been initialized?")
            .clone()
    }
}
