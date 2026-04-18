use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use crate::{errors::PermissionError, services::permission::Permissions};

/// Extracted in handlers via `ResolvedPermissions` argument.
/// Analogous to `AuthenticatedSession` but for permissions.
///
/// Contains both the computed permissions and the context they
/// were resolved for, so handlers can make further decisions.
#[derive(Clone, Debug)]
pub struct ResolvedPermissions {
    pub permissions: Permissions,
}

impl ResolvedPermissions {
    /// Check that a specific permission is present. Returns an error if not.
    pub fn require(&self, required: Permissions) -> Result<(), PermissionError> {
        if self.has(Permissions::ADMINISTRATOR) {
            return Ok(());
        }

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
}

impl<S> FromRequestParts<S> for ResolvedPermissions
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<ResolvedPermissions>()
            .cloned()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "No permissions. Missing permission middleware.",
            ))
    }
}
