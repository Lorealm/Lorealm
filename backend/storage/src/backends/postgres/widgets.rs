use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::{
    StorageError, UserWidgetStorage,
    backends::PostgresStorage,
    models::{Snowflake, UserWidget},
};

#[derive(sqlx::FromRow)]
struct PgUserWidget {
    id: Snowflake,
    user_id: Snowflake,
    name: String,
    description: String,
    source: String,
    prompt: String,
    tools: Json<Vec<Snowflake>>,
    enabled: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<PgUserWidget> for UserWidget {
    fn from(r: PgUserWidget) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            description: r.description,
            source: r.source,
            prompt: r.prompt,
            tools: r.tools.0,
            enabled: r.enabled,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl UserWidgetStorage for PostgresStorage {
    async fn list_widgets(&self, user_id: Snowflake) -> Result<Vec<UserWidget>, StorageError> {
        let rows = sqlx::query_as::<_, PgUserWidget>(
            r#"SELECT * FROM user_widgets WHERE user_id = $1 ORDER BY created_at"#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn list_enabled_widgets(
        &self,
        user_id: Snowflake,
    ) -> Result<Vec<UserWidget>, StorageError> {
        let rows = sqlx::query_as::<_, PgUserWidget>(
            "SELECT * FROM user_widgets WHERE user_id = $1 AND enabled ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_widget(&self, id: Snowflake) -> Result<Option<UserWidget>, StorageError> {
        let row = sqlx::query_as::<_, PgUserWidget>("SELECT * FROM user_widgets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    async fn save_widget(&self, widget: UserWidget) -> Result<(), StorageError> {
        sqlx::query(
            r#"INSERT INTO user_widgets
                (id, user_id, name, description, source, prompt, tools, enabled)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                source = EXCLUDED.source,
                prompt = EXCLUDED.prompt,
                tools = EXCLUDED.tools,
                enabled = EXCLUDED.enabled,
                updated_at = now()"#,
        )
        .bind(widget.id)
        .bind(widget.user_id)
        .bind(&widget.name)
        .bind(&widget.description)
        .bind(&widget.source)
        .bind(&widget.prompt)
        .bind(Json(&widget.tools))
        .bind(widget.enabled)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn set_widget_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        sqlx::query(r#"UPDATE user_widgets SET enabled = $2, updated_at = now() WHERE id = $1"#)
            .bind(id)
            .bind(enabled)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn delete_widget(&self, id: Snowflake) -> Result<(), StorageError> {
        sqlx::query(r#"DELETE FROM user_widgets WHERE id = $1"#)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
