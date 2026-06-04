use std::fmt::Display;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

impl Display for ChatRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatRole::System => write!(f, "system"),
            ChatRole::User => write!(f, "user"),
            ChatRole::Assistant => write!(f, "assistant"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Conversation {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub character_id: Snowflake,
    pub char_version_id: Snowflake,
    pub active_leaf_id: Option<Snowflake>,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ConversationMessage {
    pub id: Snowflake,
    pub conversation_id: Snowflake,
    pub parent: Option<Snowflake>,
    pub role: ChatRole,
    pub content: String,
    pub order: i32,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ConversationMetadata {
    pub id: Snowflake,
    pub character_id: Snowflake,
    pub char_version_id: Snowflake,
    pub title: Option<String>,
    pub length: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default)]
pub struct MessageEdit {
    pub content: Option<String>,
    pub metadata: Option<serde_json::Value>,
}
