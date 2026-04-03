use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Ordered list of backends to try (first available wins)
    pub backends: Vec<BackendConfig>,

    /// Whether to require postgres as fallback
    pub require_postgres_fallback: bool,

    #[cfg(feature = "redis")]
    /// Optional caching configuration
    pub cache: Option<CacheConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg(feature = "redis")]
pub struct CacheConfig {
    /// Redis connection URL
    pub redis_url: String,

    /// Default TTL for cached items in seconds
    pub ttl_seconds: u64,

    /// Key prefix for namespacing (useful for multi-tenant)
    pub key_prefix: String,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_url: "redis://localhost:6379".to_string(),
            ttl_seconds: 300, // 5 minutes default
            key_prefix: "eldoria_hub".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackendConfig {
    Postgres {
        connection_string: String,
        max_connections: u32,
    },
    Memory {
        max_size: usize,
    },
}

impl StorageConfig {
    /// Create a default config for single-user local deployment
    /// 
    /// Creates an in-memory storage with a 50MB capacity
    pub fn local_default() -> Self {
        Self {
            backends: vec![BackendConfig::Memory { max_size: 50 * 1024 * 1024 }],
            require_postgres_fallback: false,
            cache: None, // No cache for local single-user
        }
    }

    /// Create a default config for multi-user server deployment
    pub fn server_default() -> Self {
        Self {
            backends: vec![
                BackendConfig::Postgres {
                    connection_string: std::env::var("POSTGRES_URL")
                        .unwrap_or_else(|_| "postgresql://localhost/eldoria".to_string()),
                    max_connections: 10,
                },
            ],
            require_postgres_fallback: true,
            #[cfg(feature = "redis")]
            cache: Some(CacheConfig {
                redis_url: std::env::var("REDIS_URL")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                ttl_seconds: 600, // 10 minutes for production
                key_prefix: "eldoria_hub".to_string(),
            }),
        }
    }

    /// Create config from environment variables
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        // Try to load from config file first
        if let Ok(config_path) = std::env::var("STORAGE_CONFIG_PATH") {
            let config_str = std::fs::read_to_string(config_path)?;
            return Ok(serde_json::from_str(&config_str)?);
        }

        // Otherwise, determine from DEPLOYMENT_MODE
        match std::env::var("DEPLOYMENT_MODE")?.as_str() {
            "server" | "production" => Ok(Self::server_default()),
            "local" | "development" => Ok(Self::local_default()),
            mode => Err(format!("Unknown deployment mode: {mode}").into()),
        }
    }
}
