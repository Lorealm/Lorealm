use std::{env, sync::Arc, time::{Duration, Instant}};

use tokio::sync::Mutex;

use crate::{plugin_system::plugin_manager::PluginManager, resources::resource_manager::ResourceManager, systems::tokenizer::TokenizerManager, users::manager::UserManager};

#[derive(Clone)]
pub struct AppState {
    pub start_time: Instant,
    pub plugin_manager: Arc<Mutex<Option<PluginManager>>>,
    pub resource_manager: Arc<Mutex<ResourceManager>>,
    pub database_pool: Arc<Mutex<sqlx::PgPool>>,
    pub user_manager: Arc<Mutex<UserManager>>,
    pub tokenizers: Arc<Mutex<TokenizerManager>>
}

impl AppState {
    pub async fn new() -> Self {
        let postgres_db = sqlx::PgPool::connect(&get_postgres_from_env()).await.unwrap();
        sqlx::migrate!("../migrations").run(&postgres_db).await.unwrap();
        let resource_manager = Arc::new(Mutex::new(ResourceManager::new(postgres_db.clone())));
        let user_manager = Arc::new(Mutex::new(UserManager::new(postgres_db.clone(), resource_manager.lock().await.resource_dir().to_path_buf())));
        user_manager.lock().await.create_root_user().await.unwrap();
        Self {
            start_time: Instant::now(),
            plugin_manager: Arc::new(Mutex::new(Some(PluginManager::new()))),
            resource_manager,
            database_pool: Arc::new(Mutex::new(postgres_db)),
            user_manager,
            tokenizers: Arc::new(Mutex::new(TokenizerManager::new()))
        }
    }

    pub async fn initialize(&self) {
        log::info!("Initializing tokenizers");
        let tokenizer_definition = self.resource_manager.lock().await.request_builder()
            .with_credentials(Some(env::var("SYSTEM_FILE_ACCESS_TOKEN").unwrap_or("".to_string())), true)
            .with_file("res://system/tokenizers.json".to_string())
            .request_content(&self.clone())
            .await;
        
        if let Ok(mut payload) = tokenizer_definition {
            if payload.data.files.len() == 0 {
                log::warn!("No tokenizers found");
                return
            }
            let file = payload.data.files.remove(0);
            self.tokenizers.lock().await.load_definition_file(file, &self).await;
        }
    }

    pub fn get_uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    pub fn get_plugin_manager(&self) -> Arc<Mutex<Option<PluginManager>>> {
        self.plugin_manager.clone()
    }
}

fn get_postgres_from_env() -> String {
    let user = std::env::var("POSTGRES_USER").unwrap_or("postgres".to_string());
    let password = std::env::var("POSTGRES_PASSWORD").unwrap_or("root".to_string());
    let host = std::env::var("POSTGRES_HOST").unwrap_or("localhost".to_string());
    let port = std::env::var("POSTGRES_PORT").unwrap_or("5432".to_string());
    let db = std::env::var("POSTGRES_DATABASE").unwrap_or("postgres".to_string());
    format!("postgres://{}:{}@{}:{}/{}", user, password, host, port, db)
}