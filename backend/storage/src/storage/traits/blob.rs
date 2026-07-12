use std::fmt::Debug;

use async_trait::async_trait;
use bytes::Bytes;

use crate::{StorageError, models::BlobId};

#[async_trait]
pub trait BlobStore: Send + Sync + Debug {
    /// Store bytes, returning their content id. Write-if-absent: storing
    /// identical bytes twice is a no-op that returns the same id.
    async fn put(&self, bytes: &[u8]) -> Result<BlobId, StorageError>;

    /// Fetch a blob's bytes, or `None` if absent.
    async fn get(&self, id: &BlobId) -> Result<Option<Bytes>, StorageError>;

    /// Presence check without reading.
    async fn exists(&self, id: &BlobId) -> Result<bool, StorageError>;
}
