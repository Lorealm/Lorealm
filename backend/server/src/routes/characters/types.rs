use lorealm_storage::models::Snowflake;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ImportResult {
    Ok { id: Snowflake },
    Failed { reason: String },
}

impl ImportResult {
    pub fn ok(id: Snowflake) -> Self {
        Self::Ok { id }
    }

    pub fn failed(reason: impl Into<String>) -> Self {
        Self::Failed {
            reason: reason.into(),
        }
    }
}
