use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::CookieJar;
use rand::TryRngCore;
use rand::rngs::OsRng;
use sqlx::{PgPool, Row};
use thiserror::Error; // Add to deps for custom errors

use crate::state::AppState;

#[derive(Error, Debug)]
pub enum CsrfError {
    #[error("Database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("Session not found")]
    SessionNotFound,
    #[error("RNG error: {0}")]
    Rng(String),
    #[error("Internal server error")]
    Internal,
}

impl IntoResponse for CsrfError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            Self::SessionNotFound => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, self.to_string()).into_response()
    }
}

pub async fn csrf_token(
    State(state): State<AppState>,
    cookies: CookieJar,
) -> Result<String, CsrfError> {
    let auth_token = cookies
        .get("auth_token")
        .map(|c| c.value().to_string())
        .ok_or(CsrfError::SessionNotFound)?;

    get_csrf_token(Arc::new(state), auth_token).await
}

pub async  fn get_csrf_token(app_state: Arc<AppState>, auth_token: String) -> Result<String, CsrfError> { 
    let user_manager = app_state.user_manager.lock().await;
    let session = user_manager
        .get_session_id(&auth_token)
        .await
        .map_err(|_| CsrfError::SessionNotFound)?;

    drop(user_manager); // Release lock early

    let pool = app_state.database_pool.lock().await;
    let token = get_csrf_token_from_db(session, &pool).await?;

    if let Some(t) = token {
        Ok(t)
    } else {
        create_csrf_token(session, &pool).await
    }
}

async fn get_csrf_token_from_db(session: i32, pool: &PgPool) -> Result<Option<String>, CsrfError> {
    let token = sqlx::query(
        "SELECT token FROM csrf_tokens WHERE session_id = $1 AND created_at > CURRENT_TIMESTAMP - INTERVAL '15 minutes'"
    )
    .bind(session)
    .fetch_optional(pool)
    .await?
    .map(|row| row.get::<String, _>(0));

    Ok(token)
}

async fn generate_csrf_token() -> Result<String, CsrfError> {
    let mut key = [0u8; 32];
    OsRng.try_fill_bytes(&mut key).map_err(|e| CsrfError::Rng(e.to_string()))?;
    let token: String = OsRng.try_next_u32().map_err(|e| CsrfError::Rng(e.to_string()))?.to_string();
    Ok(token)
}

pub async fn create_csrf_token(session: i32, pool: &PgPool) -> Result<String, CsrfError> {
    let token = generate_csrf_token().await?;

    sqlx::query(
        "INSERT INTO csrf_tokens (session_id, token) VALUES ($1, $2)
        ON CONFLICT (session_id) DO UPDATE SET token = EXCLUDED.token, created_at = now()"
    ) // Update created_at too
    .bind(session)
    .bind(&token)
    .execute(pool)
    .await?;

    Ok(token)
}