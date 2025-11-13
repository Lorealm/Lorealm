use std::sync::Arc;

use axum_extra::extract::CookieJar;
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{api::state::context_template::templates::ContextTemplateError, state::AppState, systems::context_manager::ContextBuilder};

pub mod types;
pub mod templates;
mod entries;
mod groups;

pub fn context_template_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/", axum::routing::get(templates::list_templates))
        .route("/", axum::routing::post(templates::create_template))
        .route("/{template_id}", axum::routing::get(templates::get_template))
        .route("/{template_id}", axum::routing::patch(templates::edit_template_metadata))
        .route("/{template_id}", axum::routing::delete(templates::delete_template))
        .route("/{template_id}/groups", axum::routing::post(groups::create_group))
        .route("/{template_id}/groups/{group_id}", axum::routing::get(groups::get_group))
        .route("/{template_id}/groups/{group_id}", axum::routing::patch(groups::edit_group_metadata))
        .route("/{template_id}/groups/{group_id}", axum::routing::delete(groups::delete_group))
        .route("/{template_id}/entries", axum::routing::post(entries::create_entry))
        .route("/{template_id}/entries/{entry_id}", axum::routing::get(entries::get_entry))
        .route("/{template_id}/entries/{entry_id}", axum::routing::patch(entries::edit_entry_metadata))
        .route("/{template_id}/entries/{entry_id}", axum::routing::delete(entries::delete_entry))
        .route("/{template_id}/entries/{entry_id}/content", axum::routing::patch(entries::edit_entry_content))
        .route("/{template_id}/reorder", axum::routing::patch(templates::reorder_items))
        .with_state(state.into())
}

/// Loads a context template for a given user and template ID.
pub(super) async fn load_template(
    state: &Arc<AppState>,
    token: &str,
    template_id: Uuid,
) -> Result<ContextBuilder, ContextTemplateError> {
    let resource_manager = state.resource_manager.lock().await;
    let user_resources = resource_manager.user_resources(token, state).await?;
    let template = user_resources.load_context_template(template_id, state).await?;
    Ok(ContextBuilder::from(&template))
}

/// Extracts and validates the auth token from cookies.
async fn extract_auth_token(cookies: CookieJar) -> Result<String, ContextTemplateError> {
    cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(ContextTemplateError::StatusCode(StatusCode::UNAUTHORIZED))
}