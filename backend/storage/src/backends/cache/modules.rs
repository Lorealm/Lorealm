use async_trait::async_trait;

use crate::{
    ModuleStorage, StorageError,
    backends::CachedStorage,
    models::{CodeModule, Snowflake},
};

#[async_trait]
impl ModuleStorage for CachedStorage {
    async fn list_modules(&self, owner_id: Snowflake) -> Result<Vec<CodeModule>, StorageError> {
        self.backend.list_modules(owner_id).await
    }
    async fn get_module(&self, id: Snowflake) -> Result<Option<CodeModule>, StorageError> {
        self.backend.get_module(id).await
    }
    async fn save_module(&self, module: &CodeModule) -> Result<Snowflake, StorageError> {
        self.backend.save_module(module).await
    }
    async fn delete_module(&self, id: Snowflake) -> Result<bool, StorageError> {
        self.backend.delete_module(id).await
    }
}
