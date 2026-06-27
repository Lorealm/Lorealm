use chrono::{DateTime, Utc};
use lorealm_storage::models::Snowflake;

pub struct LoginResult {
    pub user_id: Snowflake,
    pub session_token: String,
    pub csrf_token: String,
    pub expires_at: DateTime<Utc>,
}
