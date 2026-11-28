use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::{
        StatusCode,
        header::{CACHE_CONTROL, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use lorealm_storage::models::BlobId;
use serde::Serialize;

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

#[derive(Serialize)]
pub struct StoredImage {
    pub uri: String,
}

/// Store an image a message can carry, by content address.
pub async fn put_image(
    State(state): State<Arc<AppState>>,
    _session: AuthenticatedSession,
    body: Bytes,
) -> Result<Json<StoredImage>, AppError> {
    if crate::util::image_type(&body).is_none() {
        return Err(AppError::request(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "blob.unsupported_type",
            "Only PNG, JPEG, WebP and GIF images can be attached.",
        ));
    }

    let id = state
        .services()
        .blobs()
        .put(&body)
        .await
        .map_err(|e| AppError::Internal {
            internal: e.to_string().into(),
        })?;

    Ok(Json(StoredImage { uri: id.to_uri() }))
}

fn not_found() -> AppError {
    AppError::request(
        StatusCode::NOT_FOUND,
        "blob.not_found",
        "That image no longer exists.",
    )
}
