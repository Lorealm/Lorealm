use std::sync::Arc;

use eldoria_storage::{StorageConfig, StorageManager};
use secrecy::SecretString;

use crate::{config::ConfigProvider, core::ServiceManager};

#[derive(Clone)]
pub struct AppState {
    config_provider: Arc<ConfigProvider>,
    service_provider: Arc<ServiceManager>,
    secret: SecretString,
}

impl AppState {
    pub(crate) async fn new(config: StorageConfig) -> anyhow::Result<Self> {
        let manager = StorageManager::new(config).await?;
        let config = Arc::new(ConfigProvider::load());
        let service_provider = Arc::new(ServiceManager::new(&manager, Arc::clone(&config)));

        Ok(Self {
            config_provider: config,
            service_provider,
            secret: SecretString::new(
                // TODO: CHANGE THIS
                String::from("TEMPORARY_SECRET_CHANGE_THIS").into_boxed_str(),
            ),
        })
    }

    pub(crate) fn services(&self) -> &Arc<ServiceManager> {
        &self.service_provider
    }

    pub(crate) fn config(&self) -> &Arc<ConfigProvider> {
        &self.config_provider
    }

    pub(crate) fn secret(&self) -> SecretString {
        self.secret.clone()
    }
}
