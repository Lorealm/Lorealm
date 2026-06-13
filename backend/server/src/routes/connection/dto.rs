use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::services::connection::ProfileInput;

/// NOTE: deliberately no `Debug` derive — this struct carries a plaintext
/// API key and should never be logged
#[derive(Deserialize)]
pub struct CreateSecretRequest {
    pub provider: String,
    pub label: String,
    pub key: String,
}

#[derive(Debug, Deserialize)]
pub struct RenameSecretRequest {
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct DeleteSecretResponse {
    /// Profiles that were detached (their secret_id is now null).
    pub detached_profiles: Vec<Snowflake>,
}

#[derive(Debug, Deserialize)]
pub struct ProfileRequest {
    pub secret_id: Option<Snowflake>,
    pub name: String,
    pub provider: String,
    pub model: Option<String>,
    #[serde(default)]
    pub common_params: serde_json::Value,
    #[serde(default)]
    pub provider_params: serde_json::Value,
}

#[derive(Deserialize)]
pub(super) struct SetActiveRequest {
    pub id: Option<Snowflake>,
}

impl From<ProfileRequest> for ProfileInput {
    fn from(r: ProfileRequest) -> Self {
        ProfileInput {
            secret_id: r.secret_id,
            name: r.name,
            provider: r.provider,
            model: r.model,
            common_params: r.common_params,
            provider_params: r.provider_params,
        }
    }
}
