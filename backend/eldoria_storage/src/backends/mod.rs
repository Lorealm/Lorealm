pub mod cache;
pub mod memory;
pub mod postgres;

// Re-export the supertrait so cache/mod.rs can reference it
pub(crate) use crate::Storage;

pub use cache::CachedStorage;
pub use memory::MemoryStorage;
pub use postgres::PostgresStorage;