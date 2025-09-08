use std::sync::Arc;

use crate::state::AppState;

pub mod templates;
mod entries;

pub fn context_template_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/", axum::routing::get(templates::list_templates))
        .route("/", axum::routing::post(templates::create_template))
        .route("/{template_id}", axum::routing::get(templates::get_template))
        .route("/{template_id}", axum::routing::patch(templates::update_template))
        .route("/{template_id}", axum::routing::delete(templates::delete_template))
        .with_state(state.into())
}