use std::collections::HashMap;

use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

/// Query parameters for the binary import route. The bytes ride in the request
/// body; `name` (and optionally `is_public`) come in the query string.
#[derive(Deserialize)]
pub struct ImportQuery {
    pub name: String,
    #[serde(default)]
    pub is_public: bool,
}

/// Body for the network-source route. `source` is an `hf:owner/model` coordinate;
/// `name` defaults to the coordinate when omitted.
#[derive(Deserialize)]
pub struct NetworkCreate {
    pub source: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub is_public: bool,
}

#[derive(Deserialize)]
pub struct ShareRequest {
    pub user_id: Snowflake,
}

#[derive(Deserialize)]
pub struct TokenizeRequest {
    pub content: String
}

#[derive(Deserialize)]
pub struct BatchTokenizeRequest {
    pub batch: HashMap<String, String>
}

#[derive(Serialize)]
pub struct BatchTokenizeResponse {
    pub tokenizer_id: Snowflake,
    pub counts: HashMap<String, usize>,
    pub total_tokens: usize
}