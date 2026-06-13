use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::{
    StorageError, UserToolStorage,
    backends::PostgresStorage,
    models::{Param, Snowflake, UserTool},
};

#[derive(sqlx::FromRow)]
struct PgUserTool {
    id: Snowflake,
    user_id: Snowflake,
    name: String,
    display_name: Option<String>,
    description: String,
    params: Json<Vec<Param>>,
    body: String,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PgUserTool> for UserTool {
    fn from(r: PgUserTool) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            display_name: r.display_name,
            description: r.description,
            params: r.params.0,
            body: r.body,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl UserToolStorage for PostgresStorage {
    async fn list_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, StorageError> {
        let rows = sqlx::query_as::<_, PgUserTool>(
            r#"SELECT * FROM user_tools WHERE user_id = $1 ORDER BY created_at"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_enabled_tools(&self, user_id: Snowflake) -> Result<Vec<UserTool>, StorageError> {
        let rows = sqlx::query_as::<_, PgUserTool>(
            "SELECT * FROM user_tools WHERE user_id = $1 AND enabled ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_tool(&self, id: Snowflake) -> Result<Option<UserTool>, StorageError> {
        let row = sqlx::query_as::<_, PgUserTool>("SELECT * FROM user_tools WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    async fn save_tool(&self, tool: UserTool) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO user_tools
                (id, user_id, name, display_name, description, params, body, enabled)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                display_name = EXCLUDED.display_name,
                description = EXCLUDED.description,
                params = EXCLUDED.params,
                body = EXCLUDED.body,
                enabled = EXCLUDED.enabled,
                updated_at = now()"#,
        )
        .bind(tool.id)
        .bind(tool.user_id)
        .bind(&tool.name)
        .bind(&tool.display_name)
        .bind(&tool.description)
        .bind(Json(&tool.params))
        .bind(&tool.body)
        .bind(tool.enabled)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn set_tool_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE user_tools SET enabled = $2, updated_at = now() WHERE id = $1"#)
            .bind(id)
            .bind(enabled)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_tool(&self, id: Snowflake) -> Result<(), StorageError> {
        sqlx::query(r#"DELETE FROM user_tools WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
