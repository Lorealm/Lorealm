use async_trait::async_trait;

use crate::{
    ModuleStorage, StorageError,
    backends::MemoryStorage,
    models::{CodeModule, Snowflake},
};

#[async_trait]
impl ModuleStorage for MemoryStorage {
    async fn list_modules(&self, owner_id: Snowflake) -> Result<Vec<CodeModule>, StorageError> {
        let modules = self.modules.read().await;
        let mut out: Vec<CodeModule> = modules
            .values()
            .filter(|m| m.owner_id == owner_id)
            .cloned()
            .collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    async fn get_module(&self, id: Snowflake) -> Result<Option<CodeModule>, StorageError> {
        Ok(self.modules.read().await.get(&id).cloned())
    }

    async fn save_module(&self, module: &CodeModule) -> Result<Snowflake, StorageError> {
        self.modules.write().await.insert(module.id, module.clone());
        Ok(module.id)
    }

    async fn delete_module(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.modules.write().await.remove(&id).is_some())
    }
}
