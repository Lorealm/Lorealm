use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use lorealm_storage::models::{Snowflake, User};

use super::dto::{AddMemberRequest, CreateRoleRequest, RoleResponse, UpdateRoleRequest};
use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::users::dto::UserResponse,
    services::permission::Permissions,
};

fn member(user: User) -> UserResponse {
    UserResponse {
        username: user.username,
        id: user.id,
        display_name: user.display_name,
        avatar_uri: user.avatar_uri,
        created_at: user.created_at,
    }
}

pub async fn list_roles(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
) -> Result<Json<Vec<RoleResponse>>, AppError> {
    let roles = state.services().permissions().list_roles().await?;
    Ok(Json(roles.into_iter().map(RoleResponse::from).collect()))
}

pub async fn create_role(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    let role = state
        .services()
        .permissions()
        .create_role(
            session.user_id,
            &payload.name,
            payload.permissions,
            payload.position,
        )
        .await?;
    Ok(Json(role.into()))
}

pub async fn update_role(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(role_id): Path<Snowflake>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    let role = state
        .services()
        .permissions()
        .update_role(
            session.user_id,
            role_id,
            payload.name.as_deref(),
            payload.permissions,
            payload.position,
        )
        .await?;
    Ok(Json(role.into()))
}

pub async fn delete_role(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(role_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    state
        .services()
        .permissions()
        .delete_role(session.user_id, role_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_members(
    State(state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
    Path(role_id): Path<Snowflake>,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    let users = state.services().permissions().role_members(role_id).await?;
    Ok(Json(users.into_iter().map(member).collect()))
}

pub async fn add_member(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(role_id): Path<Snowflake>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<UserResponse>, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    let user = state
        .services()
        .permissions()
        .assign_role(session.user_id, role_id, &payload.username)
        .await?;
    Ok(Json(member(user)))
}

pub async fn remove_member(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path((role_id, user_id)): Path<(Snowflake, Snowflake)>,
) -> Result<StatusCode, AppError> {
    perms.require(Permissions::MANAGE_ROLES)?;
    state
        .services()
        .permissions()
        .revoke_role(session.user_id, role_id, user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
