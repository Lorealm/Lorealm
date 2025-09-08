use std::{collections::HashMap, fmt::Display, sync::Arc};

use axum::{debug_handler, extract::{Path, State}, response::{IntoResponse, Response}, Form, Json};
use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{resources::{resource_config::ResourceError, user_resources::{self, UserResources}}, state::AppState, systems::{context_manager::ContextBuilder, context_template::{ContextError, ContextTemplate, TemplateEntry, TemplateGroup}}, users::{manager::User, user_session::UserState}};

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
#[debug_handler]
pub async fn create_template(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    session_arc: axum::Extension<Arc<RwLock<UserState>>>,
    Form(body_form): Form<CreateTemplateForm>
) -> Result<Json<ContextTemplate>, ContextTemplateError> {
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))?;
    
    let mut user_state = session_arc.write().await; // Acquire write lock
    let mut context_builder = ContextBuilder::new();
    let template = ContextTemplate::new(&body_form.name);

    context_builder.set_template(template.clone());
    context_builder.save(&state, &token).await?;
    user_state.set_context_manager(context_builder);

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

pub async fn update_template(
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
    let mut entry = user_resources.load_context_template(template_id, &state).await
        .map_err(|e| ContextTemplateError::Resource(e))?;

    drop(user_resources);
    drop(resouce_manager);

    if let Some(name) = body_form.name {
        entry.name = name;
    }
    if let Some(desc) = body_form.description {
        entry.description = Some(desc);
    }

    ContextBuilder::from(&entry).save(&state, &token).await?;

    return Ok(Json(entry));
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