use std::sync::Arc;

use reqwest::ClientBuilder;

use crate::config::AppConfig;

pub struct AppState {
    pub config: AppConfig,
    pub api_client: Arc<reqwest::Client>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let user_agent = format!(
            "Lorealm-Desktop/{} ({}; {})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            std::env::consts::ARCH,
        );
        let api_client = ClientBuilder::new()
            .user_agent(user_agent)
            .build()
            .expect("should be able to create reqwest client");
        AppState {
            config,
            api_client: Arc::new(api_client),
        }
    }
}
