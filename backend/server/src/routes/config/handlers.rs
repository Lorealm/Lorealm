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

/// Strips database credentials from the `storage` section for users who
/// can't edit the server.
fn redact_storage(storage: &mut serde_json::Value) {
    if let Some(backends) = storage.get_mut("backends").and_then(|b| b.as_array_mut()) {
        for backend in backends.iter_mut().filter_map(|b| b.as_object_mut()) {
            backend.remove("connection_string");
        }
    }
    if let Some(cache) = storage.get_mut("cache").and_then(|c| c.as_object_mut()) {
        cache.remove("redis_url");
    }
}

pub async fn get_full_config(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    perms: ResolvedPermissions,
) -> impl IntoResponse {
    let mut all = state.config().get_all();
    if perms.require(Permissions::EDIT_SERVER).is_err() {
        if let Some(storage) = all.get_mut("storage") {
            redact_storage(storage);
        }
    }
    Json(all)
}

pub async fn get_config(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(key): Path<String>,
) -> impl IntoResponse {
    let config = state.config();
    match config.get_raw(&key) {
        Some(mut value) => {
            if key == "storage" && perms.require(Permissions::EDIT_SERVER).is_err() {
                redact_storage(&mut value);
            }
            Json(value).into_response()
        }
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
