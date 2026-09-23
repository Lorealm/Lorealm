use async_trait::async_trait;

use crate::{
    BlueprintStorage, StorageError,
    backends::PostgresStorage,
    models::{Blueprint, BlueprintSummary, Snowflake},
};

#[async_trait]
impl BlueprintStorage for PostgresStorage {
    async fn list_blueprints(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<BlueprintSummary>, StorageError> {
        Ok(sqlx::query_as::<_, BlueprintSummary>(
            "SELECT id, owner_id, name, description, updated_at
             FROM blueprints WHERE owner_id = $1 ORDER BY updated_at DESC",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn get_blueprint(&self, id: Snowflake) -> Result<Option<Blueprint>, StorageError> {
        Ok(sqlx::query_as::<_, Blueprint>(
            "SELECT id, owner_id, name, description, source, created_at, updated_at
             FROM blueprints WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn save_blueprint(&self, blueprint: &Blueprint) -> Result<Snowflake, StorageError> {
        sqlx::query(
            "INSERT INTO blueprints (id, owner_id, name, description, source, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 source = EXCLUDED.source,
                 updated_at = EXCLUDED.updated_at",
        )
        .bind(blueprint.id)
        .bind(blueprint.owner_id)
        .bind(&blueprint.name)
        .bind(&blueprint.description)
        .bind(&blueprint.source)
        .bind(blueprint.created_at)
        .bind(blueprint.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(blueprint.id)
    }

    async fn delete_blueprint(&self, id: Snowflake) -> Result<bool, StorageError> {
        let done = sqlx::query("DELETE FROM blueprints WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }
}
