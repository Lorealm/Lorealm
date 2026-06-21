use std::sync::Arc;

use lorealm_storage::StorageManager;
use secrecy::SecretString;

use crate::{config::ConfigProvider, core::ServiceManager};

#[derive(Clone)]
pub struct AppState {
    config_provider: Arc<ConfigProvider>,
    service_manager: Arc<ServiceManager>,
    secret: SecretString,
}

impl AppState {
    pub(crate) async fn new() -> anyhow::Result<Self> {
        let path = std::env::var("CONFIG_PATH")
            .unwrap_or_else(|_| "config.yaml".into())
            .into();

        let config = Arc::new(ConfigProvider::load(path)?);

        let storage_config = config.get::<lorealm_storage::StorageConfig>();

        let manager = StorageManager::new(storage_config).await?;
        let service_manager = Arc::new(ServiceManager::new(&manager, Arc::clone(&config)));

        Ok(Self {
            config_provider: config,
            service_manager,
            secret: SecretString::new(
                // TODO: CHANGE THIS
                String::from("TEMPORARY_SECRET_CHANGE_THIS").into_boxed_str(),
            ),
        })
    }

    pub(crate) fn services(&self) -> &Arc<ServiceManager> {
        &self.service_manager
    }

    pub(crate) fn config(&self) -> &Arc<ConfigProvider> {
        &self.config_provider
    }

    pub(crate) fn secret(&self) -> SecretString {
        self.secret.clone()
    }
}
