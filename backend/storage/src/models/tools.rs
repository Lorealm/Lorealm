use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::models::Snowflake;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParamType {
    String,
    Number,
    Bool,
    Array(Box<ParamType>),
    Object,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub ty: ParamType,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserTool {
    pub user_id: Snowflake,
    pub name: String,
    pub id: Snowflake,
    pub display_name: Option<String>,
    pub description: String,
    pub params: Vec<Param>,
    pub body: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ParamType {
    pub fn json_schema(&self) -> serde_json::Value {
        match self {
            ParamType::String => json!({ "type": "string" }),
            ParamType::Number => json!({ "type": "number" }),
            ParamType::Bool => json!({ "type": "boolean" }),
            ParamType::Array(inner) => json!({ "type": "array", "items": inner.json_schema() }),
            ParamType::Object => json!({ "type": "object", "additionalProperties": true }),
        }
    }
}
