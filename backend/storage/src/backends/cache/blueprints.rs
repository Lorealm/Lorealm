use async_trait::async_trait;

use crate::{
    BlueprintStorage, StorageError,
    backends::CachedStorage,
    models::{Blueprint, BlueprintSummary, Snowflake},
};

#[async_trait]
impl BlueprintStorage for CachedStorage {
    async fn list_blueprints(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<BlueprintSummary>, StorageError> {
        self.backend.list_blueprints(owner_id).await
    }
    async fn get_blueprint(&self, id: Snowflake) -> Result<Option<Blueprint>, StorageError> {
        self.backend.get_blueprint(id).await
    }
    async fn save_blueprint(&self, blueprint: &Blueprint) -> Result<Snowflake, StorageError> {
        self.backend.save_blueprint(blueprint).await
    }
    async fn delete_blueprint(&self, id: Snowflake) -> Result<bool, StorageError> {
        self.backend.delete_blueprint(id).await
    }
}
