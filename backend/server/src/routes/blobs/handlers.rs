use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use lorealm_storage::models::BlobId;

use crate::{core::AppState, errors::AppError, extract::AuthenticatedSession};

const IMMUTABLE: &str = "private, max-age=31536000, immutable";

/// Serve a blob by content address.
pub async fn get_blob(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    Path(hash): Path<String>,
) -> Result<Response, AppError> {
    let id = BlobId::from_hex(&hash).ok_or_else(not_found)?;

    let bytes = state
        .services()
        .blobs()
        .get(&id)
        .await
        .map_err(|e| AppError::Internal {
            internal: e.to_string().into(),
        })?
        .ok_or_else(not_found)?;

    let content_type = infer::get(&bytes)
        .map(|t| t.mime_type())
        .unwrap_or("application/octet-stream");

    Ok((
        [(CONTENT_TYPE, content_type), (CACHE_CONTROL, IMMUTABLE)],
        bytes,
    )
        .into_response())
}

fn not_found() -> AppError {
    AppError::request(
        StatusCode::NOT_FOUND,
        "blob.not_found",
        "That image no longer exists.",
    )
}
