use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
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
