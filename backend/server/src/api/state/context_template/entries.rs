use std::sync::Arc;
use axum::{
    extract::{Path, State},
    Extension, Json,
};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    api::state::context_template::{extract_auth_token, load_template, templates::ContextTemplateError, types::ContextPosition},
    helpers::patch_variable,
    state::AppState,
    systems::{
        context_manager::ContextCoordinate,
        context_template::{ContextError, ContextRole, ContextSource, TemplateEntry},
    },
    users::user_state::UserStateManager,
};

/// Form for creating a new template entry.
#[derive(Serialize, Deserialize)]
pub struct CreateEntryForm {
    name: String,
    position: ContextPosition,
}

/// Creates a new entry in a context template.
pub async fn create_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path(template_id): Path<Uuid>,
    Json(body_form): Json<CreateEntryForm>,
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    let coords: ContextCoordinate = body_form
        .position
        .try_into()
        .map_err(|_| ContextTemplateError::InvalidPosition)?;

    let entry = TemplateEntry::new(&body_form.name);
    builder.add_entry(entry.clone(), coords)?;
    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    let formatted_entry = builder
        .get_entry(entry.id)
        .ok_or(ContextError::NotFound)?;

    Ok(Json(formatted_entry.to_owned()))
}

/// Retrieves an entry from a context template.
pub async fn get_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let builder = load_template(&state, &token, template_id).await?;

    let entry = builder.get_entry(entry_id).ok_or(ContextError::NotFound)?;
    Ok(Json(entry.to_owned()))
}

/// Form for editing entry metadata.
#[derive(Serialize, Deserialize)]
pub struct EditEntryMetadataForm {
    name: Option<String>,
    role: Option<ContextRole>,
    enabled: Option<bool>,
}

/// Edits the metadata of an existing entry.
pub async fn edit_entry_metadata(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
    Json(edit_form): Json<EditEntryMetadataForm>,
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    let entry = builder
        .get_entry_mut(entry_id)
        .ok_or(ContextError::NotFound)?;

    patch_variable(&mut entry.name, edit_form.name);
    patch_variable(&mut entry.role, edit_form.role);
    patch_variable(&mut entry.enabled, edit_form.enabled);

    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    let entry = builder.get_entry(entry_id).ok_or(ContextError::NotFound)?;
    Ok(Json(entry.to_owned()))
}

/// Deletes an entry from a context template.
pub async fn delete_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<()>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    builder.remove_entry(entry_id)?;
    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    Ok(Json(()))
}

/// Edits the content of an existing entry.
pub async fn edit_entry_content(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<Box<dyn ContextSource>>,
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    let entry = builder
        .get_entry_mut(entry_id)
        .ok_or(ContextError::NotFound)?;
    entry.set_content(payload);

    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    let entry = builder.get_entry(entry_id).ok_or(ContextError::NotFound)?;
    Ok(Json(entry.to_owned()))
}