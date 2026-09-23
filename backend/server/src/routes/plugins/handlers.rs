use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_plugins::Version;
use lorealm_storage::models::PluginLog;
use serde::Deserialize;

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    services::{
        permission::Permissions,
        plugins::{InstallReport, InstalledPlugin},
    },
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

/// The plugin's README, as Markdown.
pub async fn plugin_readme(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
) -> Result<String, AppError> {
    let readme = app_state
        .services()
        .plugins()
        .readme(session.user_id, &handle)
        .await?;

    Ok(readme)
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

#[derive(Deserialize)]
pub struct UpdatePlugin {
    /// The latest when left out.
    #[serde(default)]
    pub version: Option<Version>,
}

/// Moves the user to another installed version of the plugin.
pub async fn update_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
    Json(payload): Json<UpdatePlugin>,
) -> Result<(), AppError> {
    app_state
        .services()
        .plugins()
        .update(session.user_id, &handle, payload.version)
        .await?;

    Ok(())
}

#[derive(Deserialize)]
pub struct InstallPlugin {
    /// A plugin's source directory on the server.
    pub path: std::path::PathBuf,
}

pub async fn install_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(payload): Json<InstallPlugin>,
) -> Result<Json<InstallReport>, AppError> {
    perms.require(Permissions::ALLOW_PLUGIN_INSTALL_FROM_SOURCE)?;
    let report = app_state
        .services()
        .plugins()
        .install_from_source(session.user_id, payload.path)
        .await?;

    Ok(Json(report))
}

/// Uninstalls the plugin for everyone. Versions users are on stay until
/// they move off them.
pub async fn uninstall_plugin(
    State(app_state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
) -> Result<(), AppError> {
    perms.require(Permissions::ALLOW_PLUGIN_INSTALL_FROM_SOURCE)?;
    app_state.services().plugins().uninstall(&handle).await?;

    Ok(())
}
