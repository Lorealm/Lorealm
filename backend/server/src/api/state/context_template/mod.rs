use std::sync::Arc;

use crate::state::AppState;

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