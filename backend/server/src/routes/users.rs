use std::sync::Arc;

use axum::{extract::State, http::HeaderMap, middleware::{self, from_fn_with_state}, response::{IntoResponse, Json as ResponseJson}, Json};
use axum_extra::extract::cookie::{Cookie, SameSite};
use llm_providers::samplers::json;
use reqwest::{header, StatusCode};

use crate::{middleware::auth::auth_middleware, routes::csrf::csrf_token, state::AppState, users::manager::{CreateUserRequest, LoginRequest, Role, User, UserSettings}};

// Route handlers
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let (user, token, csrf) = state
        .user_manager
        .lock()
        .await
        .authenticate(req)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let is_secure = if cfg!(debug_assertions) { false } else { true };

    let mut cookie_builder = Cookie::build(("auth_token", token))
        .http_only(true)
        .path("/");

    if is_secure {
        cookie_builder = cookie_builder
                .secure(true)
                .same_site(SameSite::None);
        } else {
            // Local dev: avoid SameSite=None + Secure requirement by using Lax and no Secure
            cookie_builder = cookie_builder.same_site(SameSite::Lax);
    }

    let cookie = cookie_builder.build();

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    let body = serde_json::json!({
        "user": {
            "id": user.id,
            "username": user.username,
            "role": user.role
        },
        "csrfToken": csrf
    });

    Ok((headers, Json(body)))
}

pub async fn logout(
    State(state): State<AppState>,
    user: axum::Extension<User>,
) -> StatusCode {
    todo!()
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub role: String,
}

pub async fn get_current_user(
    State(state): State<AppState>,
    user: axum::Extension<User>,
) -> Result<ResponseJson<UserResponse>, StatusCode> {
    let u = UserResponse {
        id: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
    };

    Ok(ResponseJson(u))
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

pub fn protected_auth_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/csrf-token", axum::routing::get(csrf_token))
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