use std::sync::Arc;

use axum::{
    Json,
    body::Bytes,
    extract::{Path, State},
    http::header,
    response::{IntoResponse, Response},
};
use lorealm_storage::models::{Blueprint, BlueprintSummary, Realm, Snowflake};

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    routes::blueprints::dto::{
        CreateBlueprintRequest, ExpandBlueprintRequest, UpdateBlueprintRequest,
    },
    services::{
        blueprint::{
            BUNDLE_CONTENT_TYPE, BuildForm, BundlePreview, ImportedBundle, InputSpec, OutlineResult,
        },
        permission::Permissions,
    },
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

/// The introduction and the fields, for the dialog that builds a realm.
pub async fn blueprint_form(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Json<BuildForm>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .form(id, session.user_id)
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

/// The blueprint and every resource its source names directly.
pub async fn export_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(id): Path<Snowflake>,
) -> Result<Response, AppError> {
    let bundle = state
        .services()
        .blueprints()
        .export_bundle(id, session.user_id)
        .await?;
    Ok((
        [
            (header::CONTENT_TYPE, BUNDLE_CONTENT_TYPE.to_owned()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", bundle.file_name),
            ),
        ],
        bundle.bytes,
    )
        .into_response())
}

/// What a bundle would bring in. Saves nothing.
pub async fn preview_import(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    body: Bytes,
) -> Result<Json<BundlePreview>, AppError> {
    Ok(Json(
        state
            .services()
            .blueprints()
            .preview_bundle(session.user_id, &body)
            .await?,
    ))
}

pub async fn import_blueprint(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    body: Bytes,
) -> Result<Json<ImportedBundle>, AppError> {
    perms.require(Permissions::IMPORT_CONTENT)?;
    Ok(Json(
        state
            .services()
            .blueprints()
            .import_bundle(session.user_id, &body)
            .await?,
    ))
}
