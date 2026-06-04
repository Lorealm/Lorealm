use async_trait::async_trait;
use sqlx::Row;

use crate::{
    StorageError, TemplateStorage,
    backends::PostgresStorage,
    models::{Snowflake, Template, TemplateMetadata},
};

fn row_to_template(row: &sqlx::postgres::PgRow) -> Result<Template, sqlx::Error> {
    Ok(Template {
        id: row.try_get("id")?,
        owner_id: row.try_get("owner_id")?,
        name: row.try_get("name")?,
        description: row.try_get("description")?,
        nodes_json: row.try_get("nodes_json")?,
        version: row.try_get::<i32, _>("version")? as u32,
        created_at: row.try_get("created_at")?,
        updated_at: row.try_get("updated_at")?,
    })
}

fn db_err(e: sqlx::Error) -> StorageError {
    StorageError::Database(e.to_string())
}

#[async_trait]
impl TemplateStorage for PostgresStorage {
    async fn save_template(&self, template: &Template) -> Result<Snowflake, StorageError> {
        sqlx::query(
            r#"
            INSERT INTO templates
                (id, owner_id, name, description, nodes_json,
                 version, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                owner_id    = EXCLUDED.owner_id,
                name        = EXCLUDED.name,
                description = EXCLUDED.description,
                nodes_json  = EXCLUDED.nodes_json,
                version     = EXCLUDED.version,
                updated_at  = EXCLUDED.updated_at
            "#,
        )
        .bind(template.id)
        .bind(template.owner_id)
        .bind(&template.name)
        .bind(&template.description)
        .bind(&template.nodes_json)
        .bind(template.version as i32)
        .bind(template.created_at)
        .bind(template.updated_at)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(template.id)
    }

    async fn get_template(&self, id: Snowflake) -> Result<Option<Template>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_id, name, description, nodes_json,
                   version, created_at, updated_at
            FROM templates
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        row.as_ref()
            .map(row_to_template)
            .transpose()
            .map_err(db_err)
    }

    async fn get_template_owned(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<Option<Template>, StorageError> {
        let row = sqlx::query(
            r#"
            SELECT id, owner_id, name, description, nodes_json,
                   version, created_at, updated_at
            FROM templates
            WHERE id = $1 AND owner_id = $2
            "#,
        )
        .bind(id)
        .bind(owner_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(db_err)?;

        row.as_ref()
            .map(row_to_template)
            .transpose()
            .map_err(db_err)
    }

    async fn list_templates(
        &self,
        owner_id: Snowflake,
    ) -> Result<Vec<TemplateMetadata>, StorageError> {
        // Compute node_count in SQL so we don't drag full nodes_json over the wire.
        let rows = sqlx::query(
            r#"
            SELECT id, owner_id, name, description,
                   CASE WHEN jsonb_typeof(nodes_json) = 'array'
                        THEN jsonb_array_length(nodes_json)
                        ELSE 0
                   END AS node_count,
                   version, created_at, updated_at
            FROM templates
            WHERE owner_id = $1
            ORDER BY updated_at DESC
            "#,
        )
        .bind(owner_id)
        .fetch_all(&self.pool)
        .await
        .map_err(db_err)?;

        rows.iter()
            .map(|row| {
                Ok(TemplateMetadata {
                    id: row.try_get("id")?,
                    owner_id: row.try_get("owner_id")?,
                    name: row.try_get("name")?,
                    description: row.try_get("description")?,
                    node_count: row.try_get::<i32, _>("node_count")? as usize,
                    version: row.try_get::<i32, _>("version")? as u32,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                })
            })
            .collect::<Result<Vec<_>, sqlx::Error>>()
            .map_err(db_err)
    }

    async fn delete_template(
        &self,
        id: Snowflake,
        owner_id: Snowflake,
    ) -> Result<(), StorageError> {
        sqlx::query("DELETE FROM templates WHERE id = $1 AND owner_id = $2")
            .bind(id)
            .bind(owner_id)
            .execute(&self.pool)
            .await
            .map_err(db_err)?;
        Ok(())
    }

    async fn template_exists(&self, id: Snowflake) -> Result<bool, StorageError> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM templates WHERE id = $1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(db_err)?;
        Ok(exists)
    }

    async fn template_name_exists(
        &self,
        owner_id: Snowflake,
        name: &str,
        exclude_id: Option<Snowflake>,
    ) -> Result<bool, StorageError> {
        let exists: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM templates
                WHERE owner_id = $1
                  AND name = $2
                  AND ($3::BIGINT IS NULL OR id <> $3)
            )
            "#,
        )
        .bind(owner_id)
        .bind(name)
        .bind(exclude_id)
        .fetch_one(&self.pool)
        .await
        .map_err(db_err)?;
        Ok(exists)
    }

    async fn get_active_template(
        &self,
        user_id: Snowflake,
    ) -> Result<Option<Snowflake>, StorageError> {
        let active: Option<Option<Snowflake>> =
            sqlx::query_scalar("SELECT active_template_id FROM user_settings WHERE user_id = $1")
                .bind(user_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(db_err)?;

        // Outer None: no settings row. Inner None: row exists, pointer unset.
        // Both collapse to "no active template".
        Ok(active.flatten())
    }

    async fn set_active_template(
        &self,
        user_id: Snowflake,
        template_id: Option<Snowflake>,
    ) -> Result<(), StorageError> {
        // UPSERT: the migration creates no user_settings rows, so the first write
        // for a user is an INSERT and every later one an UPDATE.
        sqlx::query(
            "INSERT INTO user_settings (user_id, active_template_id) VALUES ($1, $2) \
             ON CONFLICT (user_id) DO UPDATE SET active_template_id = EXCLUDED.active_template_id",
        )
        .bind(user_id)
        .bind(template_id)
        .execute(&self.pool)
        .await
        .map_err(db_err)?;

        Ok(())
    }
}
