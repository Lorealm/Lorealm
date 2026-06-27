use std::{borrow::Cow, collections::HashMap, sync::Arc};

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use bytes::Bytes;
use lorealm_storage::models::{Snowflake, Tokenizer};
use tokio::time::Instant;

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::tokenizers::dto::{
        BatchTokenizeRequest, BatchTokenizeResponse, ImportQuery, NetworkCreate, SetActiveRequest,
        ShareRequest, TokenizeRequest,
    },
    services::permission::Permissions,
};

pub async fn list_tokenizers(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<Tokenizer>>, AppError> {
    let tokenizers = state
        .services()
        .tokenizers()
        .list_accessible(session.user_id)
        .await?;

    Ok(Json(tokenizers))
}

pub async fn get_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
) -> Result<Json<Tokenizer>, AppError> {
    let tokenizer = state
        .services()
        .tokenizers()
        .get_tokenizer(session.user_id, tokenizer_id)
        .await?;

    Ok(Json(tokenizer))
}

/// Import a tokenizer from raw bytes in the request body. `name`/`is_public`
/// come from the query string so the body stays a pure binary upload.
pub async fn import_tokenizer(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    perms: ResolvedPermissions,
    Query(query): Query<ImportQuery>,
    body: Bytes,
) -> Result<Json<Tokenizer>, AppError> {
    perms.require(Permissions::IMPORT_TOKENIZER)?;

    if body.is_empty() {
        return Err(AppError::BadRequest {
            code: "EMPTY_FILE",
            message: Cow::Borrowed("File is empty."),
        });
    }

    let tokenizer = state
        .services()
        .tokenizers()
        .ingest_tokenizer(auth.user_id, query.name, body, query.is_public)
        .await?;

    Ok(Json(tokenizer))
}

/// Create a tokenizer from a network source (`hf:owner/model`). The server
/// fetches and converts it.
pub async fn create_from_network(
    State(state): State<Arc<AppState>>,
    auth: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(request): Json<NetworkCreate>,
) -> Result<Json<Tokenizer>, AppError> {
    perms.require(Permissions::IMPORT_TOKENIZER)?;

    let tokenizer = state
        .services()
        .tokenizers()
        .ingest_from_network(
            auth.user_id,
            &request.source,
            request.name,
            request.is_public,
        )
        .await?;

    Ok(Json(tokenizer))
}

pub async fn delete_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .tokenizers()
        .delete_tokenizer(session.user_id, tokenizer_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn share_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
    Json(request): Json<ShareRequest>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .tokenizers()
        .share_tokenizer(session.user_id, tokenizer_id, request.user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn unshare_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((tokenizer_id, user_id)): Path<(Snowflake, Snowflake)>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .tokenizers()
        .unshare_tokenizer(session.user_id, tokenizer_id, user_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn list_shares(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
) -> Result<Json<Vec<Snowflake>>, AppError> {
    let users = state
        .services()
        .tokenizers()
        .list_shares(session.user_id, tokenizer_id)
        .await?;

    Ok(Json(users))
}

pub async fn count(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
    Json(request): Json<TokenizeRequest>,
) -> Result<Json<usize>, AppError> {
    let count = state
        .services()
        .tokenizers()
        .tokenize(request.content, tokenizer_id, session.user_id)
        .await?;

    Ok(Json(count.len()))
}

pub async fn batch_count(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tokenizer_id): Path<Snowflake>,
    Json(request): Json<BatchTokenizeRequest>,
) -> Result<Json<BatchTokenizeResponse>, AppError> {
    let start = Instant::now();
    let counts: HashMap<String, usize> = state
        .services()
        .tokenizers()
        .batch_tokenize(request.batch, tokenizer_id, session.user_id)
        .await?
        .into_iter()
        .map(|(k, v)| (k, v.len()))
        .collect();

    let total_tokens = counts.values().sum();

    let end = Instant::now();

    tracing::info!("Completed batch tokenization request in: {:?}", end - start);

    Ok(Json(BatchTokenizeResponse {
        tokenizer_id,
        counts,
        total_tokens: total_tokens,
    }))
}

pub async fn get_active_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Option<Snowflake>>, AppError> {
    let tok = state
        .services()
        .state()
        .get_active_tokenizer_id(session.user_id)
        .await?;

    Ok(Json(tok))
}

pub async fn set_active_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<SetActiveRequest>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_tokenizer(session.user_id, Some(request.id))
        .await?;

    Ok(())
}

pub async fn unset_active_tokenizer(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_tokenizer(session.user_id, None)
        .await?;

    Ok(())
}