use async_trait::async_trait;

use crate::{
    StorageError,
    models::{Blueprint, BlueprintSummary, Snowflake},
};

#[async_trait]
pub trait BlueprintStorage {
    async fn list_blueprints(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<BlueprintSummary>, StorageError>;

    async fn get_blueprint(&self, id: Snowflake) -> Result<Option<Blueprint>, StorageError>;

    /// Create or update a blueprint.
    async fn save_blueprint(&self, blueprint: &Blueprint) -> Result<Snowflake, StorageError>;

    /// Returns `true` if a row was deleted.
    async fn delete_blueprint(&self, id: Snowflake) -> Result<bool, StorageError>;
}
