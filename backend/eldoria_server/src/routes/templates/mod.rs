use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

mod dto;
mod handlers;

use dto::*;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_templates))
        .route("/{template_id}", get(handlers::get_template))
        .route("/sources", get(handlers::list_sources))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_template))
        .route("/{template_id}", put(handlers::update_template))
        .route("/{template_id}", delete(handlers::delete_template))
        .route("/{template_id}/nodes", post(handlers::add_node))
        .route("/{template_id}/nodes/{node_id}", put(handlers::update_node))
        .route(
            "/{template_id}/nodes/{node_id}",
            delete(handlers::delete_node),
        )
        .route(
            "/{template_id}/nodes/{node_id}/move",
            post(handlers::move_node),
        )
        .route(
            "/{template_id}/nodes/{node_id}/toggle",
            post(handlers::toggle_node),
        )
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
