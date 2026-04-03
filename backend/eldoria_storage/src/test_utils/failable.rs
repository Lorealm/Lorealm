use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{StorageCore, StorageError};

/// A wrapper around any storage backend that can be configured to fail on demand.
/// Useful for testing error handling and fallback scenarios.
#[derive(Debug)]
pub struct FailableStorage<S> {
    inner: S,
    should_fail: Arc<RwLock<bool>>,
    fail_on_init: Arc<RwLock<bool>>,
    fail_on_availability: Arc<RwLock<bool>>,
}

impl<S> FailableStorage<S> {
    pub fn new(storage: S) -> Self {
        Self {
            inner: storage,
            should_fail: Arc::new(RwLock::new(false)),
            fail_on_init: Arc::new(RwLock::new(false)),
            fail_on_availability: Arc::new(RwLock::new(false)),
        }
    }

    /// Make all operations fail
    pub async fn set_should_fail(&self, fail: bool) {
        *self.should_fail.write().await = fail;
    }

    /// Make initialization fail
    pub async fn set_fail_on_init(&self, fail: bool) {
        *self.fail_on_init.write().await = fail;
    }

    /// Make availability check fail
    pub async fn set_fail_on_availability(&self, fail: bool) {
        *self.fail_on_availability.write().await = fail;
    }

    /// Reset all failure flags
    pub async fn reset(&self) {
        *self.should_fail.write().await = false;
        *self.fail_on_init.write().await = false;
        *self.fail_on_availability.write().await = false;
    }

    async fn check_failure(&self) -> Result<(), StorageError> {
        if *self.should_fail.read().await {
            Err(StorageError::Database("Simulated failure".to_string()))
        } else {
            Ok(())
        }
    }

    /// Get access to the inner storage (useful for setup/teardown)
    pub fn inner(&self) -> &S {
        &self.inner
    }
}

#[async_trait]
impl<S: StorageCore> StorageCore for FailableStorage<S> {
    async fn is_available(&self) -> bool {
        if *self.fail_on_availability.read().await {
            return false;
        }
        self.inner.is_available().await
    }

    async fn initialize(&mut self) -> Result<(), StorageError> {
        if *self.fail_on_init.read().await {
            return Err(StorageError::Database("Simulated init failure".to_string()));
        }
        self.inner.initialize().await
    }

    fn backend_type(&self) -> &str {
        self.inner.backend_type()
    }
}

// Clone implementation if inner storage is Clone
impl<S: Clone> Clone for FailableStorage<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            should_fail: Arc::clone(&self.should_fail),
            fail_on_init: Arc::clone(&self.fail_on_init),
            fail_on_availability: Arc::clone(&self.fail_on_availability),
        }
    }
}
