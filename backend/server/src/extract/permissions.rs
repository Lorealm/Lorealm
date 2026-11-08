use std::sync::Arc;

use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{
    core::AppState,
    errors::{AppError, PermissionError},
    extract::AuthenticatedSession,
    services::permission::Permissions,
};

/// Extracted in handlers via `ResolvedPermissions` argument, after the
/// session middleware has run. Resolved on demand.
#[derive(Clone, Debug)]
pub struct ResolvedPermissions {
    pub permissions: Permissions,
}

impl ResolvedPermissions {
    /// Check that a specific permission is present. Returns an error if not.
    pub fn require(&self, required: Permissions) -> Result<(), PermissionError> {
        let mut held = self.permissions;
        if held.contains(Permissions::ADMINISTRATOR) {
            held |= Permissions::all() - Permissions::ADMIN_EXEMPT;
        }

        let missing = required - held;
        if missing.is_empty() {
            Ok(())
        } else {
            Err(PermissionError::Missing(missing))
        }
    }

    /// Convenience: does this user have a specific permission?
    pub fn has(&self, perm: Permissions) -> bool {
        self.require(perm).is_ok()
    }
}

impl FromRequestParts<Arc<AppState>> for ResolvedPermissions {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let session = AuthenticatedSession::from_request_parts(parts, state)
            .await
            .map_err(|(status, message)| AppError::request(status, "auth.unauthenticated", message))?;

        let permissions = state
            .services()
            .permissions()
            .resolve(session.user_id)
            .await?;

        Ok(Self { permissions })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holding(permissions: Permissions) -> ResolvedPermissions {
        ResolvedPermissions { permissions }
    }

    #[test]
    fn a_held_flag_passes_and_a_missing_one_names_itself() {
        let perms = holding(Permissions::IMPORT_CONTENT);
        assert!(perms.require(Permissions::IMPORT_CONTENT).is_ok());
        match perms.require(Permissions::IMPORT_CONTENT | Permissions::USE_PLUGINS) {
            Err(PermissionError::Missing(missing)) => assert_eq!(missing, Permissions::USE_PLUGINS),
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn administrator_implies_everything_but_the_exempt_flags() {
        let admin = holding(Permissions::ADMINISTRATOR);
        assert!(admin.has(Permissions::MANAGE_SERVER | Permissions::INSTALL_PLUGINS));
        assert!(!admin.has(Permissions::UNSAFE_PLUGINS));
        assert!(!admin.has(Permissions::BUILD_PLUGINS));
    }

    #[test]
    fn administrator_holding_an_exempt_flag_by_name_has_it() {
        let admin = holding(Permissions::ADMINISTRATOR | Permissions::BUILD_PLUGINS);
        assert!(admin.has(Permissions::BUILD_PLUGINS));
        assert!(!admin.has(Permissions::UNSAFE_PLUGINS));
    }
}
