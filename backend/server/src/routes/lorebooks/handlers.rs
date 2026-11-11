use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use lorealm_storage::models::{Lorebook, LorebookEntry, LorebookMetadata, Snowflake};

use crate::{
    core::AppState,
    errors::{AppError, ContextError},
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::lorebooks::dto::{ImportQuery, SetActiveLorebooksRequest, SillyTavernExport},
    services::{
        cards::{
            export::lorebook_warnings,
            lorebook::{ImportedLorebook, import_file, to_world_info},
        },
        permission::Permissions,
    },
};

/// The lorebook as a SillyTavern world file.
pub async fn export_sillytavern(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(lorebook_id): Path<Snowflake>,
) -> Result<Json<SillyTavernExport>, AppError> {
    let lorebook = state
        .services()
        .lorebooks()
        .get_lorebook(lorebook_id, session.user_id)
        .await?;
    let converted = to_world_info(&lorebook);
    Ok(Json(SillyTavernExport {
        warnings: lorebook_warnings(&lorebook, &converted),
        file: converted.book,
    }))
}

/// A SillyTavern world file or `character_book`, as a new lorebook.
pub async fn import_sillytavern(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Query(query): Query<ImportQuery>,
    Json(file): Json<serde_json::Value>,
) -> Result<Json<ImportedLorebook>, AppError> {
    perms.require(Permissions::IMPORT_CONTENT)?;
    let services = state.services();
    let (lorebook, imported) = import_file(
        &file,
        &query.name,
        session.user_id,
        services.snowflake(),
    )
    .ok_or_else(|| {
        AppError::request(
            StatusCode::BAD_REQUEST,
            "lorebook.unreadable",
            "This file isn't a lorebook Lorealm can read, or none of its entries could be read.",
        )
    })?;
    services
        .storage()
        .save_lorebook(&lorebook)
        .await
        .map_err(ContextError::from)?;
    Ok(Json(imported))
}

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
