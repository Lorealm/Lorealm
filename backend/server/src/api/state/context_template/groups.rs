use std::sync::Arc;

use axum::{extract::{Path, State}, Extension, Form, Json};
use axum_extra::extract::CookieJar;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::state::context_template::{extract_auth_token, load_template, templates::ContextTemplateError}, helpers::patch_variable, state::AppState, systems::context_template::{ContextError, TemplateGroup}, users::user_state::UserStateManager};

#[derive(Serialize, Deserialize)]
pub struct CreateGroupForm {
    name: String,
    position: Option<i32>
}

pub async fn create_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path(template_id): Path<Uuid>,
    Form(body_form): Form<CreateGroupForm>
) -> Result<Json<TemplateGroup>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    let coords = body_form.position.unwrap_or(usize::MAX as i32) as usize;

    let group = TemplateGroup::new(&body_form.name);

    builder.add_group(group.clone(), coords)?;
    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    let formatted_group = builder.get_group(group.id).unwrap();

    Ok(Json(formatted_group.to_owned()))
}

pub async fn get_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>
) -> Result<Json<TemplateGroup>, ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let builder = load_template(&state, &token, template_id).await?;

    let group = builder.get_group(group_id)
        .ok_or(ContextError::NotFound)?;

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
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>,
    Form(edit_form): Form<EditGroupForm>
) -> Result<Json<TemplateGroup>, ContextTemplateError> { 
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;
    {
        let group = builder.get_group_mut(group_id)
            .ok_or(ContextTemplateError::ContextError(ContextError::NotFound))?;

        patch_variable(&mut group.name, edit_form.name);
        patch_variable(&mut group.description, edit_form.description);
        patch_variable(&mut group.color, edit_form.color);
        patch_variable(&mut group.enabled, edit_form.enabled);
    }
    

    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    // Since `group` is no longer available, you may need to retrieve it again immutably
    let group = builder.get_group(group_id)
        .ok_or(ContextError::NotFound)?;

    Ok(Json(group.to_owned()))
}

pub async fn delete_group(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    Extension(mut state_mgr): Extension<UserStateManager>,
    Path((template_id, group_id)): Path<(Uuid, Uuid)>
) -> Result<(), ContextTemplateError> {
    let token = extract_auth_token(cookies).await?;
    let mut builder = load_template(&state, &token, template_id).await?;

    builder.remove_group(group_id)?;

    builder.save(&state, &token).await?;
    state_mgr.touch_template_id(template_id, &state, &token).await?;

    Ok(())
}