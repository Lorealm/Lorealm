mod dto;
mod handlers;

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};
pub(super) use dto::*;
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_tools))
        .route("/{tool_id}", get(handlers::get_tool))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_tool))
        .route("/{tool_id}", put(handlers::update_tool))
        .route("/{tool_id}", patch(handlers::toggle_tool_enabled))
        .route("/{tool_id}", delete(handlers::delete_tool))
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
