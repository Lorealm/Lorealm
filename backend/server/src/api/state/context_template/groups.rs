use std::sync::Arc;

use axum::{extract::{Path, State}, Form, Json};
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::state::context_template::templates::ContextTemplateError, helpers::patch_variable, state::AppState, systems::{context_manager::ContextBuilder, context_template::{ContextError, TemplateGroup}}};

#[derive(Serialize, Deserialize)]
pub struct CreateGroupForm {
    name: String,
    position: Option<i32>
}

pub async fn create_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
    Form(body_form): Form<CreateGroupForm>
) -> Result<Json<TemplateGroup>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let coords = body_form.position.unwrap_or(usize::MAX as i32) as usize;

    let group = TemplateGroup::new(&body_form.name);

    let mut builder = ContextBuilder::from(&template);
    builder.add_group(group.clone(), coords).map_err(|e| ContextTemplateError::ContextError(e))?;
    builder.save(&state, &token).await?;

    let formatted_group = builder.get_group(group.id).unwrap();

    Ok(Json(formatted_group.to_owned()))
}

pub async fn get_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>
) -> Result<Json<TemplateGroup>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let builder = ContextBuilder::from(&template);
    let group = builder.get_group(group_id)
        .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    Ok(Json(group.to_owned()))
}

#[derive(Serialize, Deserialize)]
pub struct EditGroupForm {
    name: Option<String>,
    description: Option<String>,
    color: Option<String>,
    enabled: Option<bool>
}

pub async fn edit_group_metadata(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>,
    Form(edit_form): Form<EditGroupForm>
) -> Result<Json<TemplateGroup>, ContextTemplateError> { 
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);


    let mut builder = ContextBuilder::from(&template);
    {
        let group = builder.get_group_mut(group_id)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

        patch_variable(&mut group.name, edit_form.name);
        patch_variable(&mut group.description, edit_form.description);
        patch_variable(&mut group.color, edit_form.color);
        patch_variable(&mut group.enabled, edit_form.enabled);
    }
    

    builder.save(&state, &token).await?;

    // Since `group` is no longer available, you may need to retrieve it again immutably
    let group = builder.get_group(group_id)
        .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    Ok(Json(group.to_owned()))
}

pub async fn delete_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>
) -> Result<(), ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resource_manager);

    let mut builder = ContextBuilder::from(&template);
    builder.remove_group(group_id).map_err(|e| ContextTemplateError::ContextError(e))?;

    builder.save(&state, &token).await?;

    Ok(())
}