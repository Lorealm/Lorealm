use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use lorealm_storage::models::Snowflake;
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time},
};

use crate::{
    config::UserConfig,
    core::AppState,
    errors::{AppError, AuthError},
    extract::{AuthenticatedSession, UserDevice},
    util::scaffold_user,
};

const REGISTER_DISABLED_REASON: &str = "Registrations are disabled on this server.";

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<super::dto::RegisterRequest>,
) -> Result<StatusCode, AppError> {
    // Check if the register endpoint is disabled
    if state.config().get::<UserConfig>().disable_register {
        return Err(AppError::request(
            StatusCode::FORBIDDEN,
            "auth.registration_disabled",
            REGISTER_DISABLED_REASON,
        ));
    };

    let user = state
        .services()
        .users()
        .register(payload.username, payload.email, payload.password)
        .await?;

    let config = state.config().get::<UserConfig>();

    if config.create_default_resources {
        scaffold_user(&state, &user).await?;
    }

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

async fn get_user_inner(
    state: &AppState,
    user_id: Snowflake,
) -> Result<Json<super::dto::UserResponse>, AppError> {
    let user = state.services().users().get_user(user_id).await?;
    Ok(Json(user_response(user)))
}

fn user_response(user: lorealm_storage::models::User) -> super::dto::UserResponse {
    super::dto::UserResponse {
        username: user.username,
        id: user.id,
        display_name: user.display_name,
        avatar_uri: user.avatar_uri,
        created_at: user.created_at,
    }
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<super::dto::UserResponse>, AppError> {
    get_user_inner(&state, session.user_id).await
}

pub async fn get_user(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    Path(user_id): Path<Snowflake>,
) -> Result<Json<super::dto::UserResponse>, AppError> {
    get_user_inner(&state, user_id).await
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<super::dto::UpdateProfileRequest>,
) -> Result<Json<super::dto::UserResponse>, AppError> {
    let user = match payload.display_name {
        Some(display_name) => {
            state
                .services()
                .users()
                .set_display_name(session.user_id, display_name)
                .await?
        }
        None => state.services().users().get_user(session.user_id).await?,
    };

    Ok(Json(user_response(user)))
}

pub async fn put_avatar(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    body: Bytes,
) -> Result<Json<super::dto::AvatarUpdateResponse>, AppError> {
    let kind = infer::get(&body).ok_or(AuthError::UnsupportedAvatarType)?;
    if !matches!(
        kind.mime_type(),
        "image/png" | "image/jpeg" | "image/webp" | "image/gif"
    ) {
        return Err(AuthError::UnsupportedAvatarType)?;
    }

    let avatar_uri = state
        .services()
        .users()
        .set_avatar(session.user_id, body)
        .await?;

    Ok(Json(super::dto::AvatarUpdateResponse { avatar_uri }))
}

pub async fn list_sessions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<super::dto::SessionResponse>>, AppError> {
    let sessions = state
        .services()
        .users()
        .list_sessions(session.user_id)
        .await?;

    Ok(Json(
        sessions
            .into_iter()
            .map(|s| super::dto::SessionResponse {
                current: s.id == session.session_id,
                id: s.id,
                ip_address: s.ip_address,
                user_agent: s.user_agent,
                created_at: s.created_at,
                expires_at: s.expires_at,
                last_rotated_at: s.last_rotated_at,
            })
            .collect(),
    ))
}

pub async fn revoke_session(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(session_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .users()
        .revoke_session(session.user_id, session_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_other_sessions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .users()
        .revoke_other_sessions(session.user_id, session.session_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}
