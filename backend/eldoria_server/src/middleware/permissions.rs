use std::sync::Arc;

use axum::{
    extract::{Path, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use eldoria_storage::models::Snowflake;

use crate::{
    errors::PermissionError,
    extract::AuthenticatedSession,
    services::permission::{Permissions, service::PermissionService},
};

/// Extracted in handlers via `ResolvedPermissions` argument.
/// Analogous to `AuthenticatedSession` but for permissions.
///
/// Contains both the computed permissions and the context they
/// were resolved for, so handlers can make further decisions.
#[derive(Clone, Debug)]
pub struct ResolvedPermissions {
    pub permissions: Permissions,
    pub is_owner: bool,
}

impl ResolvedPermissions {
    /// Check that a specific permission is present, returning a
    /// typed error suitable for use with `?` in handlers.
    pub fn require(&self, required: Permissions) -> Result<(), PermissionError> {
        let missing = required & !self.permissions;
        if missing.is_empty() {
            Ok(())
        } else {
            Err(PermissionError::Missing(missing))
        }
    }

    /// Convenience: does this user have a specific permission?
    pub fn has(&self, perm: Permissions) -> bool {
        self.permissions.contains(perm)
    }

    /// Convenience: is this user the server owner?
    pub fn is_owner(&self) -> bool {
        self.is_owner
    }
}

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
/// Separate from `PermissionError` (which is for missing perms) —
/// these are infrastructure/routing problems.
#[derive(Debug)]
pub enum PermissionMiddlewareError {
    /// The route is missing a required path parameter.
    MissingPathParam(&'static str),
    /// The user isn't a member of this server.
    NotAMember,
    /// The channel doesn't exist or doesn't belong to this server.
    ChannelNotFound,
    /// Something went wrong in the DB.
    Internal(String),
}

impl IntoResponse for PermissionMiddlewareError {
    fn into_response(self) -> Response {
        match self {
            Self::MissingPathParam(param) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Permission middleware misconfigured: missing `{param}` path param"),
            )
                .into_response(),
            Self::NotAMember => {
                (StatusCode::FORBIDDEN, "You are not a member of this server").into_response()
            }
            Self::ChannelNotFound => (StatusCode::NOT_FOUND, "Channel not found").into_response(),
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
    let roles = permission_service
        .get_user_roles(user_id)
        .await
        .map_err(|e| PermissionMiddlewareError::Internal(e.to_string()))?;

    if roles.is_empty() {
        return Err(PermissionMiddlewareError::NotAMember);
    }

    // Union all role permissions together
    let permissions = roles
        .iter()
        .fold(Permissions::empty(), |acc, role| acc | role.permissions);

    let is_owner = permissions.contains(Permissions::ADMINISTRATOR);

    Ok(ResolvedPermissions {
        permissions,
        is_owner,
    })
}
