use std::sync::Arc;

use reqwest::ClientBuilder;

use crate::config::AppConfig;

pub struct AppState {
    pub config: AppConfig,
    pub api_client: Arc<reqwest::Client>,
}

impl AppState {
    pub fn new(config: AppConfig) -> Self {
        let api_client = ClientBuilder::new()
            .build()
            .expect("should be able to create reqwest client");
        AppState {
            config,
            api_client: Arc::new(api_client),
        }
    }
}
