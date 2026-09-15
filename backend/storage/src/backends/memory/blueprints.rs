use async_trait::async_trait;

use crate::{
    BlueprintStorage, StorageError,
    backends::MemoryStorage,
    models::{Blueprint, BlueprintSummary, Snowflake},
};

fn summarize(b: &Blueprint) -> BlueprintSummary {
    BlueprintSummary {
        id: b.id,
        owner_id: b.owner_id,
        name: b.name.clone(),
        description: b.description.clone(),
        updated_at: b.updated_at,
    }
}

#[async_trait]
impl BlueprintStorage for MemoryStorage {
    async fn list_blueprints(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<BlueprintSummary>, StorageError> {
        let blueprints = self.blueprints.read().await;
        let mut out: Vec<BlueprintSummary> = blueprints
            .values()
            .filter(|b| b.owner_id == owner_id)
            .map(summarize)
            .collect();
        out.sort_by_key(|b| std::cmp::Reverse(b.updated_at));
        Ok(out)
    }

    async fn get_blueprint(&self, id: Snowflake) -> Result<Option<Blueprint>, StorageError> {
        Ok(self.blueprints.read().await.get(&id).cloned())
    }

    async fn save_blueprint(&self, blueprint: &Blueprint) -> Result<Snowflake, StorageError> {
        self.blueprints
            .write()
            .await
            .insert(blueprint.id, blueprint.clone());
        Ok(blueprint.id)
    }

    async fn delete_blueprint(&self, id: Snowflake) -> Result<bool, StorageError> {
        Ok(self.blueprints.write().await.remove(&id).is_some())
    }
}
