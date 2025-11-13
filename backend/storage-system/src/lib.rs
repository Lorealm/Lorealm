// Core modules
pub mod config;
pub mod error;
pub mod models;
pub mod storage;

// Backend implementations
pub mod backends {
    pub mod file;
    pub mod memory;

    #[cfg(feature = "postgres")]
    pub mod postgres;
}

// Caching layer
#[cfg(feature = "redis")]
pub mod cache;

// Re-exports for convenience
pub use config::{BackendConfig, CacheConfig, StorageConfig};
pub use error::StorageError;
pub use models::*;
pub use storage::*;

#[cfg(feature = "redis")]
pub use cache::CachedStorage;

// Test utilities (only available in tests)
#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

/// Result type alias for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        BackendConfig, Conversation, Message, MessageRole, Result, Storage, StorageConfig,
        StorageError, StorageManager, User,
    };

    #[cfg(feature = "redis")]
    pub use crate::CacheConfig;
}
