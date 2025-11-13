pub mod redis_cache;
pub use redis_cache::CachedStorage;

use async_trait::async_trait;

#[async_trait]
pub trait Cache: Send + Sync {
    async fn get(&self, key: &str) -> Option<Vec<u8>>;
    async fn set(
        &self,
        key: &str,
        value: &[u8],
        ttl_seconds: Option<u64>,
    ) -> Result<(), Box<dyn std::error::Error>>;
    async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>>;
    async fn invalidate_pattern(&self, pattern: &str) -> Result<(), Box<dyn std::error::Error>>;
}
