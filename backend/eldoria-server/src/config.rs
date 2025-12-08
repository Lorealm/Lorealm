use serde::Deserialize;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;

#[derive(Default, Deserialize, Debug)]
pub struct AppConfig {
    pub backends: Vec<storage_system::BackendConfig>,
    pub require_postgres_fallback: bool,
    pub cache: storage_system::CacheConfig,
    pub security: SecurityConfig,
    pub server: ServerConfig,
}

#[derive(Deserialize, Debug)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            cors_origins: vec![
                "http://localhost:5173".to_string(),
                "http://frontend:5173".to_string(),
            ],
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
///A 1:1 copy of tracing::Level
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Trace => write!(f, "trace"),
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: LogLevel,
}

impl ServerConfig {
    pub fn host_addr(&self) -> [u8; 4] {
        let parts: Vec<&str> = self.host.split(".").collect();
        [
            parts[0].parse().unwrap_or(0),
            parts[1].parse().unwrap_or(0),
            parts[2].parse().unwrap_or(0),
            parts[3].parse().unwrap_or(0),
        ]
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            log_level: LogLevel::Info,
        }
    }
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let config_path =
        std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config/config.dev.yaml".to_string());

    if let Ok(mut file) = File::open(&config_path) {
        let mut contents = String::new();
        if file.read_to_string(&mut contents).is_ok() {
            let config: AppConfig = serde_yaml::from_str(&contents)?;
            return Ok(config);
        }
        tracing::warn!("Failed to read config file: {config_path}");
    }

    tracing::info!("Config file not found... Using default values.");

    return Ok(AppConfig::default());
}
