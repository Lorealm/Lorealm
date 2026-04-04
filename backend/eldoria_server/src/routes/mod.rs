use std::sync::Arc;

use axum::Router;

use crate::{core::AppState, middleware::SessionMiddlewareState};

mod users;

pub async fn api_routes(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
) -> Router {
    Router::new().nest(
        "/users",
        users::router(Arc::clone(&app_state), middleware_state.clone()),
    )
}
