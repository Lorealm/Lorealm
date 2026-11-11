use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{CodeModule, Snowflake};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::modules::dto::{CreateModuleRequest, UpdateModuleRequest},
};

pub async fn list_modules(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<CodeModule>>, AppError> {
    Ok(Json(
        state.services().modules().list(session.user_id).await?,
    ))
}

pub async fn get_module(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Json<CodeModule>, AppError> {
    Ok(Json(
        state.services().modules().get(id, session.user_id).await?,
    ))
}

pub async fn create_module(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<CreateModuleRequest>,
) -> Result<Json<CodeModule>, AppError> {
    Ok(Json(
        state
            .services()
            .modules()
            .create(
                session.user_id,
                payload.name,
                payload.description,
                payload.source,
            )
            .await?,
    ))
}

pub async fn update_module(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
    Json(payload): Json<UpdateModuleRequest>,
) -> Result<Json<CodeModule>, AppError> {
    Ok(Json(
        state
            .services()
            .modules()
            .update(
                id,
                session.user_id,
                payload.name,
                payload.description,
                payload.source,
                payload.enabled,
            )
            .await?,
    ))
}

pub async fn delete_module(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .modules()
        .delete(id, session.user_id)
        .await?;
    Ok(())
}
