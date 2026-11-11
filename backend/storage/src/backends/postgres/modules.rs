use async_trait::async_trait;

use crate::{
    ModuleStorage, StorageError,
    backends::PostgresStorage,
    models::{CodeModule, Snowflake},
};

#[async_trait]
impl ModuleStorage for PostgresStorage {
    async fn list_modules(&self, owner_id: Snowflake) -> Result<Vec<CodeModule>, StorageError> {
        Ok(sqlx::query_as::<_, CodeModule>(
            "SELECT id, owner_id, name, description, source, enabled, created_at, updated_at
             FROM modules WHERE owner_id = $1 ORDER BY name",
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await?)
    }

    async fn get_module(&self, id: Snowflake) -> Result<Option<CodeModule>, StorageError> {
        Ok(sqlx::query_as::<_, CodeModule>(
            "SELECT id, owner_id, name, description, source, enabled, created_at, updated_at
             FROM modules WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn save_module(&self, module: &CodeModule) -> Result<Snowflake, StorageError> {
        sqlx::query(
            "INSERT INTO modules (id, owner_id, name, description, source, enabled, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             ON CONFLICT (id) DO UPDATE SET
                 name = EXCLUDED.name,
                 description = EXCLUDED.description,
                 source = EXCLUDED.source,
                 enabled = EXCLUDED.enabled,
                 updated_at = EXCLUDED.updated_at",
        )
        .bind(module.id)
        .bind(module.owner_id)
        .bind(&module.name)
        .bind(&module.description)
        .bind(&module.source)
        .bind(module.enabled)
        .bind(module.created_at)
        .bind(module.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(module.id)
    }

    async fn delete_module(&self, id: Snowflake) -> Result<bool, StorageError> {
        let done = sqlx::query("DELETE FROM modules WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(done.rows_affected() > 0)
    }
}
