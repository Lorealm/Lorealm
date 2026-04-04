use std::sync::{Arc, RwLock};

use eldoria_storage::{StorageConfig, StorageManager};
use secrecy::SecretString;

use crate::{config::ConfigProvider, core::ServiceProvider};

#[derive(Clone)]
pub struct AppState {
    storage_manager: Arc<RwLock<StorageManager>>,
    config_provider: Arc<ConfigProvider>,
    service_provider: Arc<ServiceProvider>,
    secret: SecretString,
}

impl AppState {
    pub(crate) async fn new(config: StorageConfig) -> anyhow::Result<Self> {
        let manager = StorageManager::new(config).await?;
        let config = Arc::new(ConfigProvider::load());
        let service_provider = Arc::new(ServiceProvider::new(&manager, Arc::clone(&config)));

        Ok(Self {
            storage_manager: Arc::new(RwLock::new(manager)),
            config_provider: config,
            service_provider,
            secret: SecretString::new(
                // TODO: CHANGE THIS
                String::from("TEMPORARY_SECRET_CHANGE_THIS").into_boxed_str(),
            ),
        })
    }

    pub(crate) fn services(&self) -> &Arc<ServiceProvider> {
        &self.service_provider
    }

    pub(crate) fn config(&self) -> &Arc<ConfigProvider> {
        &self.config_provider
    }

    pub(crate) fn secret(&self) -> SecretString {
        self.secret.clone()
    }
}
