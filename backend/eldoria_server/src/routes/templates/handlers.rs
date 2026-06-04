use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use eldoria_storage::models::{Snowflake, TemplateMetadata};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    routes::templates::SetActiveRequest,
    services::context::{ContextTemplate, SourceDefinition},
};

pub async fn list_templates(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<TemplateMetadata>>, AppError> {
    let templates = state
        .services()
        .context()
        .list_templates(session.user_id)
        .await?;

    Ok(Json(templates))
}

pub async fn get_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
) -> Result<Json<ContextTemplate>, AppError> {
    Ok(Json(
        state
            .services()
            .context()
            .get_template(template_id, session.user_id)
            .await?,
    ))
}

pub async fn create_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<super::CreateTemplateRequest>,
) -> Result<Json<ContextTemplate>, AppError> {
    let template = state
        .services()
        .context()
        .create_template(
            session.user_id,
            request.name,
            request.description,
            request.default,
        )
        .await?;

    Ok(Json(template))
}

pub async fn update_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
    Json(request): Json<super::UpdateTemplateRequest>,
) -> Result<Json<ContextTemplate>, AppError> {
    let updated_template = state
        .services()
        .context()
        .update_template(
            template_id,
            session.user_id,
            request.name,
            request.description,
        )
        .await?;

    Ok(Json(updated_template))
}

pub async fn delete_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .context()
        .delete_template(template_id, session.user_id)
        .await?;

    Ok(())
}

pub async fn list_sources(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<SourceDefinition>>, AppError> {
    let sources = state
        .services()
        .context()
        .with_registry(|reg| reg.all_sources_owned());

    Ok(Json(sources))
}

pub async fn add_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(template_id): Path<Snowflake>,
    Json(request): Json<super::CreateNodeRequest>,
) -> Result<Json<super::NodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .add_node(template_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn update_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
    Json(request): Json<super::UpdateNodeRequest>,
) -> Result<Json<super::NodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .update_node(template_id, node_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn delete_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
) -> Result<StatusCode, AppError> {
    state
        .services()
        .context()
        .delete_node(template_id, node_id, session.user_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn move_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
    Json(request): Json<super::MoveNodeRequest>,
) -> Result<Json<super::MoveNodeResponse>, AppError> {
    let response = state
        .services()
        .context()
        .move_node(template_id, node_id, session.user_id, request)
        .await?;

    Ok(Json(response.into()))
}

pub async fn toggle_node(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path((template_id, node_id)): Path<(Snowflake, Snowflake)>,
) -> Result<Json<super::ToggleNodeResponse>, AppError> {
    let changes = state
        .services()
        .context()
        .toggle_node(template_id, node_id, session.user_id)
        .await?;

    Ok(Json(super::ToggleNodeResponse { delta: changes }))
}

pub async fn get_active(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Option<Snowflake>>, AppError> {
    let template = state
        .services()
        .state()
        .get_active_template_id(session.user_id)
        .await?;

    Ok(Json(template))
}

async fn set_active(
    state: Arc<AppState>,
    session: AuthenticatedSession,
    value: Option<Snowflake>,
) -> Result<(), AppError> {
    state
        .services()
        .state()
        .set_active_template(session.user_id, value)
        .await?;
    Ok(())
}

pub async fn set_active_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<SetActiveRequest>,
) -> Result<(), AppError> {
    tracing::info!("setting active template");
    set_active(state, session, Some(request.id)).await
}

pub async fn unset_active_template(
    State(state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<(), AppError> {
    set_active(state, session, None).await
}
