use std::sync::Arc;

use crate::{api::state::context_template::context_template_routes, state::AppState};

pub mod context_template;

pub fn state_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .nest("/context_templates/", context_template_routes(state.clone()))
        .with_state(state.into())
}