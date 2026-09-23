use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{Blueprint, BlueprintSummary, Realm, Snowflake};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::blueprints::dto::{
        CreateBlueprintRequest, ExpandBlueprintRequest, UpdateBlueprintRequest,
    },
    services::blueprint::{InputSpec, OutlineResult},
};

pub async fn list_blueprints(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<BlueprintSummary>>, AppError> {
    Ok(Json(
        state.services().blueprints().list(session.user_id).await?,
    ))
}

pub async fn get_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Json<Blueprint>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .get(id, session.user_id)
            .await?,
    ))
}

pub async fn create_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<CreateBlueprintRequest>,
) -> Result<Json<Blueprint>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .create(session.user_id, payload.name, payload.source)
            .await?,
    ))
}

pub async fn update_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
    Json(payload): Json<UpdateBlueprintRequest>,
) -> Result<Json<Blueprint>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .update(
                id,
                session.user_id,
                payload.name,
                payload.description,
                payload.source,
            )
            .await?,
    ))
}

pub async fn delete_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .blueprints()
        .delete(id, session.user_id)
        .await?;
    Ok(())
}

/// What the blueprint asks for, so a client can build the form.
pub async fn blueprint_inputs(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Json<Vec<InputSpec>>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .inputs(id, session.user_id)
            .await?,
    ))
}

/// The blueprint's structure, for the editor to open it in.
pub async fn blueprint_outline(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Json<OutlineResult>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .outline(id, session.user_id)
            .await?,
    ))
}

/// Expand the blueprint into a playable realm.
pub async fn expand_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
    Json(payload): Json<ExpandBlueprintRequest>,
) -> Result<Json<Realm>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .expand(id, session.user_id, payload.inputs)
            .await?,
    ))
}
