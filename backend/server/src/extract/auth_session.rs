use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use lorealm_storage::models::Snowflake;

/// Extracted in handlers via `AuthenticatedSession` argument
#[derive(Clone)]
pub struct AuthenticatedSession {
    pub session_id: Snowflake,
    pub user_id: Snowflake,
}

impl<S> FromRequestParts<S> for AuthenticatedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedSession>()
            .cloned()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Not authenticated. Missing session middleware.",
            ))
    }
}
