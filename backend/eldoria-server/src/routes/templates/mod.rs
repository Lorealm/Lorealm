use std::sync::Arc;

use axum::{routing::{get, post, put}, Router};
use tower_cookies::CookieManagerLayer;

use crate::{app_state::AppState, middleware::authentication::{session_auth_middleware, session_csrf_middleware, SessionMiddlewareState}};

mod handlers;
mod dto;
mod error;
mod tests;

use handlers::*;

pub(super) fn templates_router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
) -> axum::Router {
    // Routes that only need session auth (GET requests)
    let session_only_routes = Router::new()
        .route("/", get(list_templates))
        .route("/{id}", get(get_template))
        .route("/sources", get(list_sources))
        .route("/{id}/validate", get(validate_template))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    // Routes that need both session and CSRF (POST/PUT/DELETE)
    let csrf_routes = Router::new()
        .route("/", post(create_template))
        .route("/{id}", put(update_template).delete(delete_template))
        .route("/{id}/nodes", post(add_node))
        .route("/{id}/nodes/{node_id}", put(update_node).delete(remove_node))
        .route("/{id}/nodes/{node_id}/move", post(move_node))
        .route("/{id}/nodes/{node_id}/toggle", post(toggle_node))
        .route("/{id}/preview", post(preview_context))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ));

    // Merge all routes
    Router::new()
        .merge(session_only_routes)
        .merge(csrf_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
