use std::sync::Arc;

use axum::{
    extract::State, http::{HeaderMap, StatusCode}, Json
};
use secrecy::ExposeSecret;
use tower_cookies::{Cookie, Cookies};
use serde::{Deserialize, Serialize};

use crate::{app_state::AppState, middleware::authentication::{AuthenticatedSession, NewCsrfToken}, models::{users::EldoriaUser, AuthError, UserDevice}, services::authentication::{error::SessionError, Session}};

#[derive(Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    csrf_token: String,
    user_id: String,
}

#[derive(Serialize)]
pub struct ProtectedResponse {
    message: String,
    user_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    new_csrf_token: Option<String>,
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AuthError> {
    let storage_lock = state.storage();
    let storage = storage_lock.write().await;
    
    let user = EldoriaUser::login(payload.username, payload.password, &storage.user_service())
        .await?;

    let user_device = UserDevice::from_http(&headers);
    
    let session = Session::new(
        user.id,
        &user_device,
        &state.server_secret().expose_secret(),
        storage.session_config().clone(),
    )
    .await
    .map_err(|e| AuthError::Session(e))?;

    let session_token = session.raw_token().ok_or(SessionError::TokenGenerationFailed)?;
    let csrf_token = session.raw_csrf_token().ok_or(SessionError::TokenGenerationFailed)?;

    storage
        .session_service()
        .save_session(session)
        .await
        .map_err(|e| AuthError::Session(SessionError::Persistence(e)))?;

    let mut cookie = Cookie::new("session_token", session_token);
    cookie.set_http_only(true);
    cookie.set_secure(true);
    cookie.set_same_site(tower_cookies::cookie::SameSite::Strict);
    cookie.set_path("/");
    cookie.set_max_age(tower_cookies::cookie::time::Duration::seconds(
        storage.session_config().ttl_seconds,
    ));
    cookies.add(cookie);
    
    drop(storage);

    Ok(Json(LoginResponse {
        csrf_token,
        user_id: user.id.to_string(),
    }))
}

// GET endpoint - only requires session authentication
pub async fn get_profile(
    auth: AuthenticatedSession,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "user_id": auth.user_id.to_string(),
        "message": "This is your profile"
    })))
}

// POST endpoint - requires both session and CSRF
pub async fn update_profile(
    auth: AuthenticatedSession,
    new_csrf: Option<axum::Extension<NewCsrfToken>>,
    Json(_payload): Json<serde_json::Value>,
) -> Result<Json<ProtectedResponse>, AuthError> {
    // Your business logic here
    
    let new_csrf_token = new_csrf.map(|ext| ext.0.0.clone());
    
    Ok(Json(ProtectedResponse {
        message: "Profile updated".to_string(),
        user_id: auth.user_id.to_string(),
        new_csrf_token, // Return new CSRF if rotated
    }))
}

pub async fn logout(
    auth: AuthenticatedSession,
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
) -> Result<StatusCode, StatusCode> {
    let storage_lock = state.storage();
    let storage = storage_lock.write().await;
    
    // Revoke session
    auth.session
        .revoke(&storage.session_service())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Remove cookie
    cookies.remove(Cookie::from("session_token"));

    Ok(StatusCode::NO_CONTENT)
}