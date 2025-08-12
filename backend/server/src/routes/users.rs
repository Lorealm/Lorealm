use std::sync::Arc;

use axum::{extract::State, middleware, response::Json as ResponseJson, Json};
use reqwest::StatusCode;

use crate::{middleware::auth::auth_middleware, state::AppState, users::manager::{CreateUserRequest, LoginRequest, Role, User, UserSettings}};

// Route handlers
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<ResponseJson<serde_json::Value>, StatusCode> {
    let (user, token) = state
        .user_manager
        .lock()
        .await
        .authenticate(req)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(ResponseJson(serde_json::json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "role": user.role
        },
        "token": token
    })))
}

pub async fn logout(
    State(state): State<AppState>,
    user: axum::Extension<User>,
) -> StatusCode {
    todo!()
}

pub async fn get_current_user(
    State(state): State<AppState>,
    user: axum::Extension<User>,
) -> Result<ResponseJson<User>, StatusCode> {
    todo!()
}

pub async fn create_user(
    State(state): State<AppState>,
    user: axum::Extension<User>,
    Json(req): Json<CreateUserRequest>,
) -> Result<ResponseJson<User>, StatusCode> {
    let creator_role = Role::from_str(&user.role)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new_user = state
        .user_manager
        .lock()
        .await
        .create_user(req, &creator_role, None)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(ResponseJson(new_user))
}

pub async fn get_user_settings(
    State(state): State<AppState>,
    user: axum::Extension<User>,
) -> Result<ResponseJson<UserSettings>, StatusCode> {
    let settings = state
        .user_manager
        .lock()
        .await
        .get_user_settings(user.id.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(ResponseJson(settings))
}

pub async fn update_user_settings(
    State(state): State<AppState>,
    user: axum::Extension<User>,
    Json(settings): Json<UserSettings>,
) -> Result<StatusCode, StatusCode> {
    state
        .user_manager
        .lock()
        .await
        .update_user_settings(user.id.to_string(), settings)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

pub fn auth_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/login", axum::routing::post(login))
        .route("/logout", axum::routing::post(logout))
        .with_state(state)
}

pub fn admin_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/users", axum::routing::post(create_user))
        .with_state(state)
}

pub fn user_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/me", axum::routing::get(get_current_user))
        .route("/settings", 
            axum::routing::get(get_user_settings)
                .put(update_user_settings)
        )
        .with_state(state)
}