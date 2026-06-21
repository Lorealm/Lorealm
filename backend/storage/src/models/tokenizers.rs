use serde::{Deserialize, Serialize};

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tokenizer {
    pub id: Snowflake,
    pub name: String,
    pub blob_uri: String,
    pub owner_id: Snowflake,
    pub is_public: bool
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TokenizerShare {
    pub tok_id: Snowflake,
    pub user_id: Snowflake
}