use async_trait::async_trait;

use crate::{
    StorageError,
    models::{CodeModule, Snowflake},
};

#[async_trait]
pub trait ModuleStorage {
    /// Every module the owner has, sources included, ordered by name.
    async fn list_modules(&self, owner_id: Snowflake) -> Result<Vec<CodeModule>, StorageError>;

    async fn get_module(&self, id: Snowflake) -> Result<Option<CodeModule>, StorageError>;

    /// Create or update a module.
    async fn save_module(&self, module: &CodeModule) -> Result<Snowflake, StorageError>;

    /// Returns `true` if a row was deleted.
    async fn delete_module(&self, id: Snowflake) -> Result<bool, StorageError>;
}
