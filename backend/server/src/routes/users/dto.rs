use crate::{services::permission::Permissions, util::double_option};
use chrono::{DateTime, Utc};
use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct LoginRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub user_id: Snowflake,
    pub csrf_token: String,
    pub session_token: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub username: String,
    pub id: Snowflake,
    pub display_name: Option<String>,
    pub avatar_uri: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default, deserialize_with = "double_option")]
    pub display_name: Option<Option<String>>,
}

#[derive(Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

/// What the signed-in user may do.
#[derive(Serialize)]
pub struct MyPermissionsResponse {
    /// Effective permission bits, as a decimal string.
    #[serde(with = "crate::routes::roles::dto::bits")]
    pub permissions: Permissions,
    /// Position of their highest role; they can manage roles below it.
    pub highest_position: i32,
}

#[derive(Serialize)]
pub struct AvatarUpdateResponse {
    pub avatar_uri: String,
}

#[derive(Serialize)]
pub struct SessionResponse {
    pub id: Snowflake,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_rotated_at: DateTime<Utc>,
    /// True for the session making the request.
    pub current: bool,
}
