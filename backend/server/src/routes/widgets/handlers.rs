use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
};
use lorealm_storage::models::{Snowflake, UserWidget};
use serde::Deserialize;

use crate::{
    core::AppState,
    errors::AppError,
    extract::AuthenticatedSession,
    services::user_widgets::{CreateUserWidget, UpdateUserWidget, builtin_widgets},
};

/// Server-shipped builtin widgets
pub async fn list_builtin_widgets(_session: AuthenticatedSession) -> Json<Vec<UserWidget>> {
    Json(builtin_widgets().to_vec())
}

pub async fn list_widgets(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
) -> Result<Json<Vec<UserWidget>>, AppError> {
    let widgets = app_state
        .services()
        .widgets()
        .list_widgets(session.user_id)
        .await?;

    Ok(Json(widgets))
}

pub async fn get_widget(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(widget_id): Path<Snowflake>,
) -> Result<Json<UserWidget>, AppError> {
    let widget = app_state
        .services()
        .widgets()
        .get_widget(widget_id, session.user_id)
        .await?;

    Ok(Json(widget))
}

pub async fn create_widget(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Json(payload): Json<CreateUserWidget>,
) -> Result<Json<UserWidget>, AppError> {
    let widget = app_state
        .services()
        .widgets()
        .create_widget(session.user_id, payload)
        .await?;

    Ok(Json(widget))
}

pub async fn update_widget(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(widget_id): Path<Snowflake>,
    Json(payload): Json<UpdateUserWidget>,
) -> Result<Json<UserWidget>, AppError> {
    let widget = app_state
        .services()
        .widgets()
        .update_widget(widget_id, session.user_id, payload)
        .await?;

    Ok(Json(widget))
}

#[derive(Deserialize)]
pub struct ToggleEnabled {
    pub enabled: bool,
}

pub async fn toggle_widget_enabled(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(widget_id): Path<Snowflake>,
    Json(payload): Json<ToggleEnabled>,
) -> Result<(), AppError> {
    app_state
        .services()
        .widgets()
        .set_enabled(widget_id, session.user_id, payload.enabled)
        .await?;

    Ok(())
}

pub async fn delete_widget(
    State(app_state): State<Arc<AppState>>,
    session: AuthenticatedSession,
    Path(widget_id): Path<Snowflake>,
) -> Result<(), AppError> {
    app_state
        .services()
        .widgets()
        .delete_widget(widget_id, session.user_id)
        .await?;

    Ok(())
}
