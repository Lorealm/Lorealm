use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::{
    StorageError, UserWidgetStorage,
    backends::SqliteStorage,
    models::{Snowflake, UserWidget, WidgetMode, WidgetPlacement},
};
use super::NOW;

#[derive(sqlx::FromRow)]
struct SqliteUserWidget {
    id: Snowflake,
    user_id: Snowflake,
    name: String,
    description: String,
    source: String,
    prompt: String,
    render: String,
    tools: Json<Vec<Snowflake>>,
    enabled: bool,
    placement: WidgetPlacement,
    mode: WidgetMode,
    props: Json<Vec<String>>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SqliteUserWidget> for UserWidget {
    fn from(r: SqliteUserWidget) -> Self {
        Self {
            id: r.id,
            user_id: r.user_id,
            name: r.name,
            description: r.description,
            source: r.source,
            prompt: r.prompt,
            render: r.render,
            tools: r.tools.0,
            enabled: r.enabled,
            placement: r.placement,
            mode: r.mode,
            props: r.props.0,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

#[async_trait]
impl UserWidgetStorage for SqliteStorage {
    async fn list_widgets(&self, user_id: Snowflake) -> Result<Vec<UserWidget>, StorageError> {
        let rows = sqlx::query_as::<_, SqliteUserWidget>(
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
        let rows = sqlx::query_as::<_, SqliteUserWidget>(
            "SELECT * FROM user_widgets WHERE user_id = $1 AND enabled ORDER BY created_at",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn get_widget(&self, id: Snowflake) -> Result<Option<UserWidget>, StorageError> {
        let row = sqlx::query_as::<_, SqliteUserWidget>("SELECT * FROM user_widgets WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Into::into))
    }

    async fn save_widget(&self, widget: UserWidget) -> Result<(), StorageError> {
        sqlx::query(
            &format!(r#"INSERT INTO user_widgets
                (id, user_id, name, description, source, prompt, render, tools,
                 enabled, placement, mode, props)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            ON CONFLICT (id) DO UPDATE SET
                name = EXCLUDED.name,
                description = EXCLUDED.description,
                source = EXCLUDED.source,
                prompt = EXCLUDED.prompt,
                render = EXCLUDED.render,
                tools = EXCLUDED.tools,
                enabled = EXCLUDED.enabled,
                placement = EXCLUDED.placement,
                mode = EXCLUDED.mode,
                props = EXCLUDED.props,
                updated_at = {NOW}"#),
        )
        .bind(widget.id)
        .bind(widget.user_id)
        .bind(&widget.name)
        .bind(&widget.description)
        .bind(&widget.source)
        .bind(&widget.prompt)
        .bind(&widget.render)
        .bind(Json(&widget.tools))
        .bind(widget.enabled)
        .bind(widget.placement)
        .bind(widget.mode)
        .bind(Json(&widget.props))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn set_widget_enabled(&self, id: Snowflake, enabled: bool) -> Result<(), StorageError> {
        sqlx::query(&format!(r#"UPDATE user_widgets SET enabled = $2, updated_at = {NOW} WHERE id = $1"#))
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
