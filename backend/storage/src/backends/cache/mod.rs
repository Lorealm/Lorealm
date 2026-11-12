pub mod blueprints;
pub mod characters;
pub mod connection;
pub mod conversation;
pub mod lorebook;
pub mod modules;
pub mod permissions;
pub mod persona;
pub mod plugins;
pub mod realms;
pub mod sessions;
pub mod state;
pub mod templates;
pub mod tokenizer;
pub mod tools;
pub mod users;
pub mod widgets;

use crate::error::StorageError;
use crate::{Storage, StorageCore};
use async_trait::async_trait;
use redis::Client;
use serde::{Serialize, de::DeserializeOwned};
use std::sync::Arc;

#[derive(Debug)]
pub struct CachedStorage {
    backend: Arc<dyn Storage>,
    redis: Client,
    ttl: u64,
    prefix: String,
}

impl CachedStorage {
    pub fn new(
        backend: Arc<dyn Storage>,
        redis_url: &str,
        ttl: u64,
        prefix: impl Into<String>,
    ) -> Result<Self, StorageError> {
        let redis = Client::open(redis_url)
            .map_err(|e| StorageError::InvalidInput(format!("Redis connection failed: {e}")))?;

        Ok(Self {
            backend,
            redis,
            ttl,
            prefix: prefix.into(),
        })
    }

    pub(crate) fn key(&self, entity: &str, id: &str) -> String {
        format!("{}:{}:{}", self.prefix, entity, id)
    }

    pub(crate) async fn cache_get<T: DeserializeOwned>(&self, key: &str) -> Option<T> {
        let mut conn = self.redis.get_multiplexed_async_connection().await.ok()?;
        let bytes: Vec<u8> = redis::AsyncCommands::get(&mut conn, key).await.ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    pub(crate) async fn cache_set<T: Serialize>(&self, key: &str, value: &T) {
        if let Ok(mut conn) = self.redis.get_multiplexed_async_connection().await
            && let Ok(bytes) = serde_json::to_vec(value)
        {
            let _: Result<(), _> =
                redis::AsyncCommands::set_ex(&mut conn, key, bytes, self.ttl).await;
        }
    }

    pub(crate) async fn cache_del(&self, key: &str) {
        if let Ok(mut conn) = self.redis.get_multiplexed_async_connection().await {
            let _: Result<(), _> = redis::AsyncCommands::del(&mut conn, key).await;
        }
    }

    pub(crate) async fn get_through<T, F>(
        &self,
        key: &str,
        fetch: F,
    ) -> Result<Option<T>, StorageError>
    where
        T: Serialize + DeserializeOwned,
        F: std::future::Future<Output = Result<Option<T>, StorageError>>,
    {
        if let Some(cached) = self.cache_get::<T>(key).await {
            return Ok(Some(cached));
        }

        let result = fetch.await?;

        if let Some(ref value) = result {
            self.cache_set(key, value).await;
        }

        Ok(result)
    }
}

#[async_trait]
impl StorageCore for CachedStorage {
    async fn is_available(&self) -> bool {
        self.backend.is_available().await
            && self.redis.get_multiplexed_async_connection().await.is_ok()
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        Err(StorageError::InvalidInput(
            "Cannot initialize through cache wrapper — initialize the backend directly".into(),
        ))
    }

    fn backend_type(&self) -> &str {
        "cached"
    }
}
