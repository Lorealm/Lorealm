use lorealm_storage::models::{ConnectionProfile, Snowflake};
use secrecy::zeroize::Zeroizing;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct ProfileInput {
    /// None for keyless endpoints (e.g. self-hosted).
    pub secret_id: Option<Snowflake>,
    pub name: String,
    pub model: Option<String>,
    pub provider: String,
    pub common_params: serde_json::Value,
    pub provider_params: serde_json::Value,
}

pub struct ResolvedConnection {
    pub profile: ConnectionProfile,
    pub api_key: Option<Zeroizing<String>>,
}

/// Write-side DTO for common parameters. Deliberately narrower than the
/// provider crate's `CommonParams` (no tools — those are runtime, not
/// profile, state) and strict about unknown fields to catch typos.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct CommonParamsInput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
}
