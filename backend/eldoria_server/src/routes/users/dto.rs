use serde::{Deserialize, Serialize};
use eldoria_storage::models::Snowflake;

// ----- Login -----

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

// ----- Register -----

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub password: String,
}

// ----- User -----

#[derive(Serialize)]
pub struct UserResponse {
    pub username: String,
    pub id: Snowflake,
}
