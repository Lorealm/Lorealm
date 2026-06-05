use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;

use crate::models::Snowflake;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Character {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub current_version_id: Snowflake,

    pub metadata: CharacterMetadata,
    pub definition: CharacterDefinition,
    pub lorebook_id: Option<Snowflake>,

    // CCv3 round-trip preservation. Opaque JSON blob of anything
    // we didn't recognize on import. Never consumed internally.
    pub extensions: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CharacterVersionInfo {
    pub id: Snowflake,
    pub version_num: u32,
    pub card_version: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CharacterMetadata {
    pub display_name: Option<String>, // defaults to definition.name
    pub alt_names: Vec<String>,       // for search/sort
    pub tags: Vec<String>,
    pub creator: String,
    pub creator_notes: String,
    pub card_version: String, // the HUMAN-facing version string ("1.2.0")
    pub avatar_uri: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct CharacterDefinition {
    pub name: String, // in-chat name, {{char}} substitution target
    pub description: String,
    pub personality: String,
    pub scenario: String,
    pub system_prompt: String,
    pub post_history_instructions: String,
    pub example_messages: Vec<ExampleDialogue>,
    pub greetings: Vec<Greeting>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Greeting {
    pub id: Snowflake,
    pub order: u32,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct ExampleDialogue {
    pub id: Snowflake,
    pub order: u32,
    pub content: String,
}

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct CharacterSummary {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub name: String,
    pub display_name: Option<String>,
    pub tags: Vec<String>,
    pub avatar_uri: Option<String>,
    pub card_version: String,
    pub lorebook_id: Option<Snowflake>,
    pub updated_at: DateTime<Utc>,
}
