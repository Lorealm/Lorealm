use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::PluginLog;
use serde::Deserialize;

use crate::{
    core::AppState, errors::AppError, extract::AuthenticatedSession,
    services::plugins::InstalledPlugin,
};

pub async fn list_plugins(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<InstalledPlugin>>, AppError> {
    let plugins = app_state.services().plugins().list(session.user_id).await?;

    Ok(Json(plugins))
}

pub async fn list_plugin_logs(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
) -> Result<Json<Vec<PluginLog>>, AppError> {
    let logs = app_state
        .services()
        .plugins()
        .logs(session.user_id, &handle)
        .await?;

    Ok(Json(logs))
}

#[derive(Deserialize)]
pub struct ToggleEnabled {
    pub enabled: bool,
}

pub async fn toggle_plugin_enabled(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
    Json(payload): Json<ToggleEnabled>,
) -> Result<(), AppError> {
    let plugins = app_state.services().plugins();
    if payload.enabled {
        plugins.enable(session.user_id, &handle).await?;
    } else {
        plugins.disable(session.user_id, &handle).await?;
    }

    Ok(())
}

#[derive(Deserialize)]
pub struct PluginOrder {
    /// Every enabled plugin's handle, in the order to load them.
    pub order: Vec<String>,
}

pub async fn set_plugin_order(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<PluginOrder>,
) -> Result<(), AppError> {
    app_state
        .services()
        .plugins()
        .set_order(session.user_id, payload.order)
        .await?;

    Ok(())
}
