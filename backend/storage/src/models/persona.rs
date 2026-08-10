use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPersona {
    pub id: Snowflake,
    pub user_id: Snowflake,
    pub name: String,
    pub display_name: Option<String>,
    pub pseudonym: Option<String>,
    pub avatar_uri: Option<String>,
    pub description: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// Binds a character to a persona other than the user's active one.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersonaOverride {
    pub character_id: Snowflake,
    pub persona_id: Snowflake,
    pub user_id: Snowflake,
}
