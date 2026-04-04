use eldoria_storage::models::Snowflake;

/// Extracted in handlers via `AuthenticatedSession` argument
#[derive(Clone)]
pub struct AuthenticatedSession {
    pub session_id: Snowflake,
    pub user_id: Snowflake,
}
