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
    services::{
        commands::{CommandRun, RunCommandRequest},
        tools::{CreateUserTool, UpdateUserTool, builtin_tools},
    },
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

/// The subset callable as `/name` from the composer.
pub async fn list_commands(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<UserTool>>, AppError> {
    let commands = app_state
        .services()
        .commands()
        .list(session.user_id)
        .await?;

    Ok(Json(commands))
}

pub async fn run_command(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(tool_id): Path<Snowflake>,
    Json(request): Json<RunCommandRequest>,
) -> Result<Json<CommandRun>, AppError> {
    let run = app_state
        .services()
        .commands()
        .run(
            session.user_id,
            request.conversation_id,
            tool_id,
            request.args,
        )
        .await?;

    Ok(Json(run))
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
