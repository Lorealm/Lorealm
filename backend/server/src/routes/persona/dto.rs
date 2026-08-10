use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct AvatarUpdateResponse {
    pub avatar_uri: String,
}

#[derive(Deserialize)]
pub(super) struct SetActiveRequest {
    pub id: Snowflake,
}

#[derive(Deserialize)]
pub(super) struct SetOverrideRequest {
    pub persona_id: Snowflake,
}
