#[cfg(feature = "redis")]
use crate::CacheConfig;
#[cfg(feature = "redis")]
use crate::storage::Storage;
use crate::{BackendConfig, StorageConfig, StorageError};
use crate::{
    BlobStore,
    backends::{
        blob::FsBlobStore, memory::MemoryStorage, postgres::PostgresStorage, sqlite::SqliteStorage,
    },
};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Clone)]
pub struct StorageManager {
    /// The active storage backend
    active_backend: Arc<dyn Storage>,
    blob_store: Arc<dyn BlobStore>,
    /// Name of the active backend for logging
    backend_name: String,
    /// Whether caching is enabled
    cache_enabled: bool,
}

impl StorageManager {
    /// Create a new storage manager with fallback logic and optional caching
    pub async fn new(config: StorageConfig) -> Result<Self, StorageError> {
        let mut last_error = None;

        for backend_config in &config.backends {
            info!(
                "Attempting to initialize storage backend: {:?}",
                backend_config
            );

            match Self::create_backend(backend_config).await {
                Ok(mut backend) => {
                    if backend.is_available().await {
                        match backend.initialize().await {
                            Ok(_) => {
                                let backend_name = backend.backend_type().to_string();
                                info!("Successfully initialized {} storage backend", backend_name);

                                let backend_arc: Arc<dyn Storage> = Arc::from(backend);

                                // Optionally wrap with cache
                                let (final_backend, cache_enabled) = if let Some(ref cache_config) =
                                    config.cache
                                {
                                    #[cfg(feature = "redis")]
                                    match Self::wrap_with_cache(backend_arc.clone(), cache_config)
                                        .await
                                    {
                                        Ok(cached_backend) => {
                                            info!("Caching layer enabled with Redis");
                                            (cached_backend, true)
                                        }
                                        Err(e) => {
                                            warn!(
                                                "Failed to enable caching layer: {}. Continuing without cache.",
                                                e
                                            );
                                            (backend_arc, false)
                                        }
                                    }
                                } else {
                                    (backend_arc, false)
                                };

                                return Ok(Self {
                                    active_backend: final_backend,
                                    blob_store: Arc::new(
                                        FsBlobStore::scaffold(config.data_directory).await?,
                                    ),
                                    backend_name,
                                    cache_enabled,
                                });
                            }
                            Err(e) => {
                                warn!("Failed to initialize backend: {}", e);
                                last_error = Some(e);
                            }
                        }
                    } else {
                        warn!("Backend is not available");
                    }
                }
                Err(e) => {
                    warn!("Failed to create backend: {}", e);
                    last_error = Some(e);
                }
            }
        }

        // If we get here, all backends failed
        error!("All storage backends failed to initialize");
        Err(last_error.unwrap_or_else(|| {
            StorageError::Configuration("No storage backends configured".to_string())
        }))
    }

    /// Create a backend from config
    async fn create_backend(config: &BackendConfig) -> Result<Box<dyn Storage>, StorageError> {
        match config {
            BackendConfig::Postgres {
                connection_string,
                max_connections,
                create_automatically,
            } => {
                let url = connection_string
                    .clone()
                    .or_else(|| std::env::var("LOREALM_POSTGRES_URL").ok())
                    .ok_or(StorageError::ConnectionError(
                        "missing postgres url".to_string(),
                    ))?;
                Ok(Box::new(
                    PostgresStorage::new(&url, *max_connections, *create_automatically).await?,
                ) as Box<dyn Storage>)
            }
            BackendConfig::Sqlite {
                path,
                max_connections,
                create_automatically,
            } => Ok(Box::new(
                SqliteStorage::new(path, *max_connections, *create_automatically).await?,
            ) as Box<dyn Storage>),
            BackendConfig::Memory { max_size: _ } => {
                Ok(Box::new(MemoryStorage::new()) as Box<dyn Storage>)
            }
        }
    }

    /// Wrap a storage backend with Redis caching
    #[cfg(feature = "redis")]
    async fn wrap_with_cache(
        backend: Arc<dyn Storage>,
        cache_config: &CacheConfig,
    ) -> Result<Arc<dyn Storage>, StorageError> {
        use crate::{backends::CachedStorage, storage::StorageCore};

        let cache = CachedStorage::new(
            backend,
            &cache_config.redis_url,
            cache_config.ttl_seconds,
            &cache_config.key_prefix,
        )
        .await?;

        // Verify Redis is actually accessible
        if !cache.is_available().await {
            return Err(StorageError::BackendUnavailable(
                "Redis cache is not accessible".to_string(),
            ));
        }

        Ok(Arc::new(cache))
    }

    /// Get the active storage backend
    pub fn storage(&self) -> Arc<dyn Storage> {
        Arc::clone(&self.active_backend)
    }

    pub fn blob(&self) -> Arc<dyn BlobStore> {
        Arc::clone(&self.blob_store)
    }

    /// Get the name of the active backend
    pub fn backend_name(&self) -> &str {
        &self.backend_name
    }

    /// Check if caching is enabled
    pub fn is_cache_enabled(&self) -> bool {
        self.cache_enabled
    }
}

impl std::ops::Deref for StorageManager {
    type Target = dyn Storage;

    fn deref(&self) -> &Self::Target {
        &*self.active_backend
    }
}
