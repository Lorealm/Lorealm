use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use reqwest::StatusCode;

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    services::permission::Permissions,
};

pub async fn get_full_config(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
) -> impl IntoResponse {
    Json(state.config().get_all())
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let config = state.config();
    match config.get_raw(&key) {
        Some(value) => Json(value).into_response(),
        None => (StatusCode::NOT_FOUND, "unknown config key").into_response(),
    }
}

pub async fn patch_config(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(key): Path<String>,
    Json(partial): Json<serde_json::Value>,
) -> Result<impl IntoResponse, AppError> {
    perms.require(Permissions::EDIT_SERVER)?;
    let config = state.config();
    let value = config.patch_by_key(&key, partial)?;

    Ok(Json(value).into_response())
}
