use std::sync::Arc;

use crate::state::AppState;

pub mod create;

pub fn context_template_routes(state: AppState) -> axum::Router<Arc<AppState>> {
    axum::Router::new()
        .route("/create", axum::routing::get(create::create_template))
        .with_state(state.into())
}