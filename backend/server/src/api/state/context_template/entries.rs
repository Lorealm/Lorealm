use std::sync::Arc;
use axum::{extract::{Path, State}, Form, Json};
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use crate::{api::state::context_template::{templates::ContextTemplateError, types::ContextPosition}, helpers::patch_variable, state::AppState, systems::{context_manager::{ContextBuilder, ContextCoordinate}, context_template::{ContextError, ContextRole, ContextSource, TemplateEntry}}};

// Modified CreateEntryForm to use the Position type
#[derive(Serialize, Deserialize)]
pub struct CreateEntryForm {
    name: String,
    position: ContextPosition, // Use the custom Position type
}

pub async fn create_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
    Form(body_form): Form<CreateEntryForm>,
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager
        .user_resources(&token, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources
        .load_context_template(template_id, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let coords: ContextCoordinate = body_form
        .position
        .try_into()
        .map_err(|e| ContextTemplateError::ContextError(e))?;

    let entry = TemplateEntry::new(&body_form.name);

    let mut builder = ContextBuilder::from(&template);
    builder
        .add_entry(entry.clone(), coords)
        .map_err(|e| ContextTemplateError::ContextError(e))?;
    builder.save(&state, &token).await?;

    let formatted_entry = builder.get_entry(entry.id).unwrap();

    Ok(Json(formatted_entry.to_owned()))
}

pub async fn get_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager
        .user_resources(&token, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources
        .load_context_template(template_id, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let builder = ContextBuilder::from(&template);
    let entry = builder.get_entry(entry_id)
        .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    return Ok(Json(entry.to_owned()));
}

#[derive(Serialize, Deserialize)]
pub struct EditEntryMetadataForm {
    name: Option<String>,
    role: Option<ContextRole>,
    enabled: Option<bool>
}

pub async fn edit_entry_metadata(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
    Form(edit_form): Form<EditEntryMetadataForm>
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager
        .user_resources(&token, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources
        .load_context_template(template_id, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let mut builder = ContextBuilder::from(&template);
    {
        let entry = builder.get_entry_mut(entry_id)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

        patch_variable(&mut entry.name, edit_form.name);
        patch_variable(&mut entry.role, edit_form.role);
        patch_variable(&mut entry.enabled, edit_form.enabled);
    }

    builder.save(&state, &token).await?;
    let entry = builder.get_entry(entry_id)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    return Ok(Json(entry.to_owned()));
}

pub async fn delete_entry(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>
) -> Result<(), ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager
        .user_resources(&token, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources
        .load_context_template(template_id, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let mut builder = ContextBuilder::from(&template);
    builder.remove_entry(entry_id)
        .map_err(|e| ContextTemplateError::ContextError(e))?;

    builder.save(&state, &token).await?;

    Ok(())
}

pub async fn edit_entry_content(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, entry_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<Box<dyn ContextSource>>
) -> Result<Json<TemplateEntry>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager
        .user_resources(&token, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources
        .load_context_template(template_id, &state)
        .await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let mut builder = ContextBuilder::from(&template);
    {
        let entry = builder.get_entry_mut(entry_id)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;
        entry.set_content(payload);
    }

    builder.save(&state, &token).await?;
    let entry = builder.get_entry(entry_id)
        .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    Ok(Json(entry.to_owned()))
}