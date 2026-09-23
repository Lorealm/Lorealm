use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::PluginLog;
use serde::Deserialize;

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    services::{permission::Permissions, plugins::InstalledPlugin},
};

pub async fn list_plugins(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
) -> Result<Json<Vec<InstalledPlugin>>, AppError> {
    let plugins = app_state
        .services()
        .plugins()
        .list(session.user_id, allows_unsafe(&perms))
        .await?;

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

/// One of the plugin's declared frontend entries: HTML, or for an unsafe slot,
/// a module. The client reads it as text either way.
pub async fn frontend_entry(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path((handle, entry)): Path<(String, String)>,
) -> Result<String, AppError> {
    let entry = app_state
        .services()
        .plugins()
        .frontend_entry(session.user_id, &handle, &entry, allows_unsafe(&perms))
        .await?;

    Ok(entry)
}

fn allows_unsafe(perms: &ResolvedPermissions) -> bool {
    perms.require(Permissions::ALLOW_UNSAFE_PLUGINS).is_ok()
}

#[derive(Deserialize)]
pub struct PluginCall {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// A call from the plugin's frontend to its backend.
pub async fn call_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
    Json(payload): Json<PluginCall>,
) -> Result<Json<serde_json::Value>, AppError> {
    let value = app_state
        .services()
        .plugins()
        .call_frontend(session.user_id, &handle, payload.name, payload.args)
        .await?;

    Ok(Json(value))
}
