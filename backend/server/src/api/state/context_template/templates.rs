use std::{collections::HashMap, sync::Arc};

use axum::{extract::{Path, State}, response::{IntoResponse, Response}, Form, Json};
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::state::context_template::types::ContextPosition, helpers::patch_variable, resources::resource_config::ResourceError, state::AppState, systems::{context_manager::{ContextBuilder, ContextCoordinate}, context_template::{ContextError, ContextTemplate}}};

#[derive(Debug, thiserror::Error)]
pub enum ContextTemplateError {
    #[error("Resource Error: {0}")]
    Resource(ResourceError),
    #[error("Status Code: {0}")]
    StatusCode(StatusCode),
     #[error("Context Error: {0}")]
    ContextError(ContextError)
}

impl IntoResponse for ContextTemplateError {
    fn into_response(self) -> Response {
        match self {
            ContextTemplateError::Resource(resource_error) => resource_error.to_string().into_response(),
            ContextTemplateError::StatusCode(status_code) => return status_code.into_response(),
            ContextTemplateError::ContextError(context_error) => context_error.to_string().into_response(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CreateTemplateForm {
    name: String
}

pub async fn create_template(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Form(body_form): Form<CreateTemplateForm>
) -> Result<Json<ContextTemplate>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;
    
    let mut context_builder = ContextBuilder::new();
    let template = ContextTemplate::new(&body_form.name);

    context_builder.set_template(template.clone());
    context_builder.save(&state, &token).await?;

    Ok(Json::from(template))
}

#[derive(Serialize, Deserialize)]
pub struct TemplateListResponse {
    templates: HashMap<Uuid, String>
}

pub async fn list_templates(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>
) -> Result<Json<TemplateListResponse>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resouce_manager = state.resource_manager.lock().await;
    let user_resources = resouce_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let list_result = user_resources.list_context_templates(&state).await;

    let mut templates = HashMap::new();

    match list_result {
        Ok(files) => {
            for template in files {
                templates.insert(template.id, template.name);
            }
        },
        Err(e) => return Err(ContextTemplateError::Resource(e)),
    }
    

    return Ok(Json(TemplateListResponse{ templates }));
}

pub async fn get_template(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>
) -> Result<Json<ContextTemplate>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resouce_manager = state.resource_manager.lock().await;
    let user_resources = resouce_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let entry = user_resources.load_context_template(template_id, &state).await;

    match entry {
        Ok(template) => Ok(Json(template)),
        Err(e) => Err(ContextTemplateError::Resource(e))
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateTemplateForm {
    name: Option<String>,
    description: Option<String>
}

pub async fn edit_template_metadata(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
    Form(body_form): Form<UpdateTemplateForm>
) -> Result<Json<ContextTemplate>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resouce_manager = state.resource_manager.lock().await;
    let user_resources = resouce_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let mut template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resouce_manager);

    patch_variable(&mut template.name, body_form.name);
    patch_variable(&mut template.description, body_form.description);

    ContextBuilder::from(&template).save(&state, &token).await?;

    return Ok(Json(template));
}

pub async fn delete_template(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>
) -> Result<StatusCode, StatusCode> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let resouce_manager = state.resource_manager.lock().await;
    let user_resources = resouce_manager.user_resources(&token, &state).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match user_resources.delete_context_template(template_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(_) => Err(StatusCode::NO_CONTENT),
    }
}

#[derive(Deserialize)]
pub struct ReorderItemsForm {
    from: ContextPosition,
    to: ContextPosition
}

pub async fn reorder_items(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path(template_id): Path<Uuid>,
    Form(reorder_form): Form<ReorderItemsForm>
) -> Result<Json<ContextTemplate>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;

    let resouce_manager = state.resource_manager.lock().await;
    let user_resources = resouce_manager.user_resources(&token, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;
    let template = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resouce_manager);

    let from_coords: ContextCoordinate = reorder_form
        .from
        .try_into()
        .map_err(|e| ContextTemplateError::ContextError(e))?;

    let to_coords: ContextCoordinate = reorder_form
        .to
        .try_into()
        .map_err(|e| ContextTemplateError::ContextError(e))?;

    let mut builder = ContextBuilder::from(&template);
    
    // if inner ID is present, that means it's always an entry
    if let Some(_) = from_coords.inner_id {
        let entry_id = builder.get_entry_by_coordinate(&from_coords)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?
            .id;

        builder.move_entry(entry_id, to_coords)
            .map_err(|e| ContextTemplateError::ContextError(e))?;
    }
    else {
        let (id, item_type) = match builder.get_group_by_coordinate(&from_coords) {
            Some(group) => (group.id, "group"),
            None => {
                match builder.get_entry_by_coordinate(&from_coords) {
                    Some(entry) => (entry.id, "entry"),
                    None => return Err(ContextTemplateError::ContextError(ContextError::NotFound)),
                }
            },
        };

        match item_type {
            "group" => builder.move_group(id, to_coords),
            "entry" => builder.move_entry(id, to_coords),
            _ => return Err(ContextTemplateError::ContextError(ContextError::NotFound))
        }.map_err(|e| ContextTemplateError::ContextError(e))?;
    }

    builder.save(&state, &token).await?;
    
    let template = builder.get_template()
        .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

    Ok(Json(template.to_owned()))
}