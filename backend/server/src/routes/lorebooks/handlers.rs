use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{Lorebook, LorebookEntry, LorebookMetadata, Snowflake};

use crate::{
    core::AppState, errors::AppError, extract::AuthenticatedSession,
    routes::lorebooks::dto::SetActiveLorebooksRequest,
};

pub async fn get_lorebook(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(lorebook_id): Path<Snowflake>,
) -> Result<Json<Lorebook>, AppError> {
    let lorebook = state
        .services()
        .lorebooks()
        .get_lorebook(lorebook_id, session.user_id)
        .await?;

    Ok(Json(lorebook))
}

pub async fn list_lorebooks(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<LorebookMetadata>>, AppError> {
    let entries = state
        .services()
        .lorebooks()
        .list_lorebooks(session.user_id)
        .await?;

    Ok(Json(entries))
}

pub async fn create_lorebook(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::CreateLorebookRequest>,
) -> Result<Json<Lorebook>, AppError> {
    let lorebook = state
        .services()
        .lorebooks()
        .create_lorebook(session.user_id, request.into())
        .await?;

    Ok(Json(lorebook))
}

pub async fn update_lorebook(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(lorebook_id): Path<Snowflake>,
    Json(request): Json<super::UpdateLorebookRequest>,
) -> Result<Json<Lorebook>, AppError> {
    let lorebook = state
        .services()
        .lorebooks()
        .update_lorebook(lorebook_id, session.user_id, request.into())
        .await?;

    Ok(Json(lorebook))
}

pub async fn delete_lorebook(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(lorebook_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .lorebooks()
        .delete_lorebook(lorebook_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn get_entry(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((lorebook_id, entry_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<LorebookEntry>, AppError> {
    let entry = state
        .services()
        .lorebooks()
        .get_entry(lorebook_id, session.user_id, entry_id)
        .await?;

    Ok(Json(entry))
}

pub async fn create_entry(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(lorebook_id): Path<Snowflake>,
) -> Result<Json<LorebookEntry>, AppError> {
    let entry = state
        .services()
        .lorebooks()
        .create_entry(lorebook_id, session.user_id)
        .await?;

    Ok(Json(entry))
}

pub async fn update_entry(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((lorebook_id, entry_id)): Path<(Snowflake, Snowflake)>,
    Json(request): Json<super::UpdateEntryRequest>,
) -> Result<Json<LorebookEntry>, AppError> {
    let entry = state
        .services()
        .lorebooks()
        .update_entry(lorebook_id, entry_id, session.user_id, request.source)
        .await?;

    Ok(Json(entry))
}

pub async fn delete_entry(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((lorebook_id, entry_id)): Path<(Snowflake, Snowflake)>,
) -> Result<(), AppError> {
    state
        .services()
        .lorebooks()
        .delete_entry(lorebook_id, entry_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn get_active(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Option<Vec<Snowflake>>>, AppError> {
    let lorebooks = state
        .services()
        .state()
        .get_active_lorebook_ids(session.user_id)
        .await?;

    Ok(Json(lorebooks))
}

pub async fn set_active(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<SetActiveLorebooksRequest>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_lorebooks(session.user_id, request.ids)
        .await?;

    Ok(())
}
