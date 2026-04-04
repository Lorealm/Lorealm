use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time},
};

use crate::{
    config::UserConfig,
    core::AppState,
    errors::{AppError, AuthError},
    extract::UserDevice,
};

const REGISTER_DISABLED_REASON: &str = "Registration is disabled on this server.";

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<super::dto::RegisterRequest>,
) -> Result<StatusCode, AppError> {
    // Check if the register endpoint is disabled
    if state.config().get::<UserConfig>().disable_register {
        return Err(AppError::Forbidden {
            code: "REGISTER_DISABLED",
            message: REGISTER_DISABLED_REASON.into(),
        });
    };

    state
        .services()
        .users()
        .register(payload.username, payload.email, payload.password)
        .await?;

    Ok(StatusCode::OK)
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    user_device: UserDevice,
    Json(payload): Json<super::dto::LoginRequest>,
) -> Result<Json<super::dto::LoginResponse>, AppError> {
    if payload.email.is_none() && payload.username.is_none() {
        return Err(AuthError::MalformedCredentials.into());
    }

    let secret = state.secret();

    let result = state
        .services()
        .users()
        .login(
            payload.email,
            payload.username,
            payload.password,
            &secret,
            user_device,
        )
        .await?;

    let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);

    cookies.add(
        Cookie::build(("session_token", result.session_token.clone()))
            .path("/")
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::seconds(max_age))
            .build(),
    );

    cookies.add(
        Cookie::build(("csrf_token", result.csrf_token.clone()))
            .path("/")
            .http_only(false)
            .secure(true)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::seconds(max_age))
            .build(),
    );

    Ok(Json(super::dto::LoginResponse {
        user_id: result.user_id,
        csrf_token: result.csrf_token,
        session_token: result.session_token,
    }))
}
