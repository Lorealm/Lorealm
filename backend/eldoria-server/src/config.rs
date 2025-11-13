use serde::Deserialize;
use std::fmt::Display;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Debug)]
pub struct AppConfig {
    pub backends: Vec<storage_system::BackendConfig>,
    pub require_postgres_fallback: bool,
    pub cache: storage_system::CacheConfig,
    pub security: SecurityConfig,
    pub server: ServerConfig
}

#[derive(Deserialize, Debug)]
pub struct SecurityConfig {
    pub cors_origins: Vec<String>
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
    pub log_level: LogLevel
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

pub fn load_config() -> anyhow::Result<AppConfig> {
    let mut file = File::open("config/config.dev.yaml")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let config: AppConfig = serde_yaml::from_str(&contents)?;
    Ok(config)
}