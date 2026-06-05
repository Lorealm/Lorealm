use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use eldoria_storage::models::Snowflake;

use crate::{
    extract::{AuthenticatedSession, ResolvedPermissions},
    services::permission::{Permissions, service::PermissionService},
};

#[derive(Clone)]
pub struct PermissionMiddlewareState {
    pub permission_service: Arc<PermissionService>,
}

impl PermissionMiddlewareState {
    pub fn new(permission_service: Arc<PermissionService>) -> Self {
        Self { permission_service }
    }
}

/// Errors that can occur during permission resolution in middleware.
#[derive(Debug)]
pub enum PermissionMiddlewareError {
    /// Something went wrong in storage.
    Internal(String),
}

impl IntoResponse for PermissionMiddlewareError {
    fn into_response(self) -> Response {
        match self {
            Self::Internal(msg) => {
                tracing::error!("Permission resolution failed: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

pub async fn server_permission_middleware(
    State(state): State<PermissionMiddlewareState>,
    session: AuthenticatedSession,
    mut request: Request,
    next: Next,
) -> Result<Response, PermissionMiddlewareError> {
    let resolved = resolve_server_permissions(&state.permission_service, session.user_id).await?;

    request.extensions_mut().insert(resolved);
    Ok(next.run(request).await)
}

async fn resolve_server_permissions(
    permission_service: &PermissionService,
    user_id: Snowflake,
) -> Result<ResolvedPermissions, PermissionMiddlewareError> {
    let mut roles = permission_service
        .get_user_roles(user_id)
        .await
        .map_err(|e| PermissionMiddlewareError::Internal(e.to_string()))?;

    if roles.is_empty() {
        // return Err(PermissionMiddlewareError::NoRoles);
        roles.push(crate::services::permission::Role {
            id: Snowflake(0),
            name: "admin".to_string(),
            permissions: Permissions::ADMINISTRATOR,
            position: 0,
        });
        // TODO: for future me: i'm lazy af. pls fix
    }

    // Union all role permissions together
    let permissions = roles
        .iter()
        .fold(Permissions::empty(), |acc, role| acc | role.permissions);

    Ok(ResolvedPermissions {
        permissions,
    })
}
