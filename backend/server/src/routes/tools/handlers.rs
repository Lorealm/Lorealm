use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{Snowflake, UserTool};

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    services::tools::{CreateUserTool, UpdateUserTool, builtin_tools},
};

/// Server-shipped builtin tools
pub async fn list_builtin_tools(_session: AuthenticatedSession) -> Json<Vec<UserTool>> {
    Json(builtin_tools().to_vec())
}

pub async fn list_tools(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<UserTool>>, AppError> {
    let tools = app_state
        .services()
        .tools()
        .list_tools(session.user_id)
        .await?;

    Ok(Json(tools))
}

pub async fn get_tool(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tool_id): Path<Snowflake>,
) -> Result<Json<UserTool>, AppError> {
    let tool = app_state
        .services()
        .tools()
        .get_tool(tool_id, session.user_id)
        .await?;

    Ok(Json(tool))
}

pub async fn create_tool(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(request): Json<CreateUserTool>,
) -> Result<Json<Snowflake>, AppError> {
    let tool = app_state
        .services()
        .tools()
        .create_tool(session.user_id, request)
        .await?;

    Ok(Json(tool))
}

pub async fn update_tool(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tool_id): Path<Snowflake>,
    Json(request): Json<UpdateUserTool>,
) -> Result<Json<UserTool>, AppError> {
    let updated = app_state
        .services()
        .tools()
        .update_tool(tool_id, session.user_id, request)
        .await?;

    Ok(Json(updated))
}

pub async fn toggle_tool_enabled(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tool_id): Path<Snowflake>,
    Json(request): Json<super::ToolEnabledRequest>,
) -> Result<(), AppError> {
    app_state
        .services()
        .tools()
        .set_enabled(tool_id, session.user_id, request.enabled)
        .await?;

    Ok(())
}

pub async fn delete_tool(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tool_id): Path<Snowflake>,
) -> Result<(), AppError> {
    app_state
        .services()
        .tools()
        .delete_tool(tool_id, session.user_id)
        .await?;

    Ok(())
}
