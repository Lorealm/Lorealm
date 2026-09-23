mod handlers;

use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new().route("/", get(handlers::list_plugins)).layer(
        axum::middleware::from_fn_with_state(middleware_state.clone(), session_auth_middleware),
    );

    let csrf_routes = Router::new()
        .route("/{handle}", patch(handlers::toggle_plugin_enabled))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ));

    Router::new()
        .merge(session_only_routes)
        .merge(csrf_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
