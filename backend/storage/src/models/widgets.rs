use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

/// How a [`UserWidget`] reaches the conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WidgetPlacement {
    /// Always on screen, fed a slice of weaver state.
    #[default]
    Widget,
    /// Placed inline by the model through the `attach` tool.
    Attachment,
}

/// Which messages a `Widget`-placement row renders on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type, Default)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WidgetMode {
    #[default]
    Latest,
    Every,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWidget {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub name: String,
    pub description: String,
    pub source: String,
    pub prompt: String,
    pub tools: Vec<Snowflake>,
    pub enabled: bool,
    #[serde(default)]
    pub placement: WidgetPlacement,
    #[serde(default)]
    pub mode: WidgetMode,
    #[serde(default)]
    pub props: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
