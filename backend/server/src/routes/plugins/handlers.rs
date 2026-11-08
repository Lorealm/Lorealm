use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    response::sse::{Event, KeepAlive, Sse},
};
use futures_util::{Stream, StreamExt};
use lorealm_plugins::Version;
use lorealm_storage::models::{PluginLog, Snowflake};
use serde::Deserialize;

use crate::{
    core::AppState,
    errors::AppError,
    extract::{AuthenticatedSession, ResolvedPermissions},
    services::{
        permission::Permissions,
        plugins::{InstallReport, InstalledPlugin, PluginSettings},
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
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
    Json(payload): Json<ToggleEnabled>,
) -> Result<(), AppError> {
    let plugins = app_state.services().plugins();
    if payload.enabled {
        perms.require(Permissions::USE_PLUGINS)?;
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
    perms: ResolvedPermissions,
    Json(payload): Json<PluginOrder>,
) -> Result<(), AppError> {
    perms.require(Permissions::USE_PLUGINS)?;
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

pub async fn plugin_settings(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(handle): Path<String>,
) -> Result<Json<PluginSettings>, AppError> {
    let settings = app_state
        .services()
        .plugins()
        .settings(session.user_id, &handle)
        .await?;

    Ok(Json(settings))
}

pub async fn save_plugin_settings(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
    Json(values): Json<serde_json::Map<String, serde_json::Value>>,
) -> Result<(), AppError> {
    perms.require(Permissions::USE_PLUGINS)?;
    app_state
        .services()
        .plugins()
        .save_settings(session.user_id, &handle, values)
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
    perms.has(Permissions::UNSAFE_PLUGINS)
}

#[derive(Deserialize)]
pub struct PluginCall {
    pub name: String,
    #[serde(default)]
    pub args: serde_json::Value,
    /// The conversation the slot is showing, if any.
    #[serde(default)]
    pub conversation: Option<Snowflake>,
    /// The message the slot sits under, if any.
    #[serde(default)]
    pub message: Option<Snowflake>,
    #[serde(default)]
    pub blocking: bool,
}

/// A call from the plugin's frontend to its backend.
pub async fn call_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
    Json(payload): Json<PluginCall>,
) -> Result<Json<serde_json::Value>, AppError> {
    perms.require(Permissions::USE_PLUGINS)?;
    let value = app_state
        .services()
        .plugins()
        .call_frontend(
            session.user_id,
            &handle,
            payload.name,
            payload.args,
            payload.conversation,
            payload.message,
            payload.blocking,
        )
        .await?;

    Ok(Json(value))
}

/// What the user's plugins send their frontends, as they send it.
pub async fn frontend_events(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    let events = app_state
        .services()
        .plugins()
        .frontend_events(session.user_id)
        .map(|event| Event::default().json_data(event));

    Sse::new(events).keep_alive(KeepAlive::default())
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
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
    Json(payload): Json<UpdatePlugin>,
) -> Result<(), AppError> {
    perms.require(Permissions::USE_PLUGINS)?;
    app_state
        .services()
        .plugins()
        .update(session.user_id, &handle, payload.version)
        .await?;

    Ok(())
}

#[derive(Deserialize)]
pub struct InstallPlugin {
    /// A folder on the server or a link; the server works out how to install
    /// from it.
    pub location: String,
    /// Keeps the folder linked, so the plugin can be reloaded from it.
    #[serde(default)]
    pub dev: bool,
}

pub async fn install_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Json(payload): Json<InstallPlugin>,
) -> Result<Json<InstallReport>, AppError> {
    let plugins = app_state.services().plugins();
    let location = payload.location.trim();
    if payload.dev {
        perms.require(Permissions::BUILD_PLUGINS)?;
        return Ok(Json(plugins.install_dev(session.user_id, location).await?));
    }
    require_install(&perms)?;
    let may_build = perms
        .require(Permissions::BUILD_PLUGINS)
        .is_ok();
    let report = plugins
        .install(session.user_id, location, may_build)
        .await?;

    Ok(Json(report))
}

/// Builds a plugin installed for development from its folder again and
/// reloads it for everyone.
pub async fn reload_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
) -> Result<Json<InstallReport>, AppError> {
    perms.require(Permissions::BUILD_PLUGINS)?;
    let report = app_state
        .services()
        .plugins()
        .reload(session.user_id, &handle)
        .await?;

    Ok(Json(report))
}

/// Installs the zipped plugin in the body, building it if it isn't built.
pub async fn upload_plugin(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    perms: ResolvedPermissions,
    body: axum::body::Bytes,
) -> Result<Json<InstallReport>, AppError> {
    require_install(&perms)?;
    let may_build = perms
        .require(Permissions::BUILD_PLUGINS)
        .is_ok();
    let report = app_state
        .services()
        .plugins()
        .install_zip(session.user_id, body.to_vec(), may_build)
        .await?;

    Ok(Json(report))
}

/// Installing pre-built plugins; building them allows it too.
fn require_install(perms: &ResolvedPermissions) -> Result<(), AppError> {
    perms
        .require(Permissions::INSTALL_PLUGINS)
        .or_else(|_| perms.require(Permissions::BUILD_PLUGINS))?;
    Ok(())
}

/// Uninstalls the plugin for everyone. Versions users are on stay until
/// they move off them.
pub async fn uninstall_plugin(
    State(app_state): State<Arc<AppState>>,
    perms: ResolvedPermissions,
    Path(handle): Path<String>,
) -> Result<(), AppError> {
    require_install(&perms)?;
    app_state.services().plugins().uninstall(&handle).await?;

    Ok(())
}
