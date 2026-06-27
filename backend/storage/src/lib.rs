// Core modules
pub mod config;
pub mod error;
pub mod models;
pub mod storage;

// Backend implementations
pub mod backends;

// Re-exports for convenience
pub use config::{BackendConfig, CacheConfig, StorageConfig};
pub use error::StorageError;
pub use storage::*;

// Test utilities (only available in tests)
#[cfg(test)]
pub mod test_utils;

/// Result type alias for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        BackendConfig, Result, Storage, StorageConfig, StorageError, StorageManager, models::*,
    };

    #[cfg(feature = "redis")]
    pub use crate::CacheConfig;
}
