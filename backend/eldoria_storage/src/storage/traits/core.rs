use crate::StorageError;
use async_trait::async_trait;

/// Core trait - must be implemented by all backends
#[async_trait]
pub trait StorageCore: Send + Sync + std::fmt::Debug {
    async fn is_available(&self) -> bool;
    async fn initialize(&mut self) -> Result<(), StorageError>;
    fn backend_type(&self) -> &str;
}
