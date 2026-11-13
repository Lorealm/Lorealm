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
    config::{HostingConfig, UserConfig},
    core::AppState,
    errors::{AppError, AuthError},
    extract::{AuthenticatedSession, ResolvedPermissions, UserDevice},
    services::permission::Permissions,
    util::scaffold_user,
};

const REGISTER_DISABLED_REASON: &str = "Registrations are disabled on this server.";

pub async fn register(
    State(state): State<Arc<AppState>>,
    user_device: UserDevice,
    Json(payload): Json<super::dto::RegisterRequest>,
) -> Result<StatusCode, AppError> {
    // Check if the register endpoint is disabled. An invite gets around it.
    if state.config().get::<UserConfig>().disable_register && payload.invite_code.is_none() {
        return Err(AppError::request(
            StatusCode::FORBIDDEN,
            "auth.registration_disabled",
            REGISTER_DISABLED_REASON,
        ));
    };

    state
        .services()
        .rate_limits()
        .register_attempt(user_device.ip)?;

    let users = state.services().users();
    let user = match payload.invite_code {
        Some(code) => {
            users
                .register_with_invite(payload.username, payload.email, payload.password, &code)
                .await?
        }
        None => {
            users
                .register(payload.username, payload.email, payload.password)
                .await?
        }
    };

    state
        .services()
        .permissions()
        .claim_administrator_if_unclaimed(user.id)
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

    let limits = state.services().rate_limits();
    let account = payload
        .username
        .clone()
        .or_else(|| payload.email.clone())
        .unwrap_or_default();
    limits.login_attempt(user_device.ip, &account)?;

    let secret = state.secret();

    let result = match state
        .services()
        .users()
        .login(
            payload.email,
            payload.username,
            payload.password,
            &secret,
            user_device,
        )
        .await
    {
        Ok(result) => result,
        Err(e @ (AuthError::UserNotFound | AuthError::IncorrectPassword)) => {
            limits.login_failed(&account);
            return Err(e.into());
        }
        Err(e) => return Err(e.into()),
    };

    let max_age = (result.expires_at - Utc::now()).num_seconds().max(0);
    let cookie_secure = state.config().get::<HostingConfig>().cookie_secure;

    cookies.add(
        Cookie::build(("session_token", result.session_token.clone()))
            .path("/")
            .http_only(true)
            .secure(cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::seconds(max_age))
            .build(),
    );

    cookies.add(
        Cookie::build(("csrf_token", result.csrf_token.clone()))
            .path("/")
            .http_only(false)
            .secure(cookie_secure)
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

pub async fn my_permissions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<super::dto::MyPermissionsResponse>, AppError> {
    let actor = state
        .services()
        .permissions()
        .actor(session.user_id)
        .await?;

    Ok(Json(super::dto::MyPermissionsResponse {
        permissions: actor.held,
        highest_position: actor.top,
    }))
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
        Err(AuthError::UnsupportedAvatarType)?;
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

pub async fn change_password(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<super::dto::ChangePasswordRequest>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .users()
        .change_password(
            session.user_id,
            session.session_id,
            payload.current_password,
            payload.new_password,
        )
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Revoke the session making the request and clear its cookies.
pub async fn logout(
    State(state): State<Arc<AppState>>,
    cookies: Cookies,
    session: AuthenticatedSession,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .sessions()
        .revoke_session(session.session_id)
        .await?;

    for name in ["session_token", "csrf_token"] {
        cookies.remove(Cookie::build((name, "")).path("/").build());
    }

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

pub async fn list_users(
    State(state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
) -> Result<Json<Vec<super::dto::AdminUserResponse>>, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    let users = state.services().users().list_users().await?;
    Ok(Json(users.into_iter().map(Into::into).collect()))
}

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
    Json(payload): Json<super::dto::CreateUserRequest>,
) -> Result<Json<super::dto::AdminUserResponse>, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    let user = state
        .services()
        .users()
        .register(payload.username, payload.email, payload.password)
        .await?;

    if state.config().get::<UserConfig>().create_default_resources {
        scaffold_user(&state, &user).await?;
    }

    Ok(Json(user.into()))
}

pub async fn admin_update_user(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(user_id): Path<Snowflake>,
    Json(payload): Json<super::dto::AdminUpdateUserRequest>,
) -> Result<Json<super::dto::AdminUserResponse>, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    state
        .services()
        .permissions()
        .check_may_manage_user(session.user_id, user_id)
        .await?;

    let user = state
        .services()
        .users()
        .admin_update(
            user_id,
            payload.email,
            payload.display_name,
            payload.password,
        )
        .await?;
    Ok(Json(user.into()))
}

pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(user_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    state
        .services()
        .permissions()
        .check_may_delete_user(session.user_id, user_id)
        .await?;

    state.services().users().delete_user(user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn revoke_user_sessions(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(user_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    state
        .services()
        .permissions()
        .check_may_manage_user(session.user_id, user_id)
        .await?;

    state
        .services()
        .users()
        .revoke_all_sessions(user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_invites(
    State(state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
) -> Result<Json<Vec<super::dto::InviteResponse>>, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    let invites = state.services().users().list_invites().await?;
    Ok(Json(invites.into_iter().map(Into::into).collect()))
}

pub async fn create_invite(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(payload): Json<super::dto::CreateInviteRequest>,
) -> Result<Json<super::dto::CreatedInviteResponse>, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    // Same rule as giving the role to someone directly.
    if let Some(role_id) = payload.role_id {
        perms.require(Permissions::MANAGE_ROLES)?;
        state
            .services()
            .permissions()
            .check_may_assign(session.user_id, role_id)
            .await?;
    }

    let (invite, code) = state
        .services()
        .users()
        .create_invite(
            session.user_id,
            payload.role_id,
            payload.max_uses,
            payload.expires_at,
        )
        .await?;

    Ok(Json(super::dto::CreatedInviteResponse {
        invite: invite.into(),
        code,
    }))
}

pub async fn delete_invite(
    State(state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
    Path(invite_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    perms.require(Permissions::MANAGE_USERS)?;
    state.services().users().delete_invite(invite_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
