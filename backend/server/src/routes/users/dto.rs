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
    /// Lets the user register while registration is disabled.
    #[serde(default)]
    pub invite_code: Option<String>,
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

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    #[serde(default)]
    pub email: Option<String>,
    pub password: String,
}

#[derive(Deserialize)]
pub struct AdminUpdateUserRequest {
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub display_name: Option<Option<String>>,
    /// Signs the user out everywhere.
    #[serde(default)]
    pub password: Option<String>,
}

/// A user as seen by someone who manages accounts.
#[derive(Serialize)]
pub struct AdminUserResponse {
    pub id: Snowflake,
    pub username: String,
    pub email: Option<String>,
    pub display_name: Option<String>,
    pub avatar_uri: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<lorealm_storage::models::User> for AdminUserResponse {
    fn from(user: lorealm_storage::models::User) -> Self {
        Self {
            id: user.id,
            username: user.username,
            email: user.email,
            display_name: user.display_name,
            avatar_uri: user.avatar_uri,
            created_at: user.created_at,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateInviteRequest {
    #[serde(default)]
    pub role_id: Option<Snowflake>,
    /// Omit for unlimited uses.
    #[serde(default)]
    pub max_uses: Option<i32>,
    /// Omit for an invite that never expires.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub id: Snowflake,
    pub role_id: Option<Snowflake>,
    pub created_by: Option<Snowflake>,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<lorealm_storage::models::Invite> for InviteResponse {
    fn from(invite: lorealm_storage::models::Invite) -> Self {
        Self {
            id: invite.id,
            role_id: invite.role_id,
            created_by: invite.created_by,
            max_uses: invite.max_uses,
            uses: invite.uses,
            expires_at: invite.expires_at,
            created_at: invite.created_at,
        }
    }
}

/// The plain code is only ever returned here.
#[derive(Serialize)]
pub struct CreatedInviteResponse {
    #[serde(flatten)]
    pub invite: InviteResponse,
    pub code: String,
}
