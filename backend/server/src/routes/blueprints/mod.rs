use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch, post},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_blueprints))
        .route("/{blueprint_id}", get(handlers::get_blueprint))
        .route("/{blueprint_id}/inputs", get(handlers::blueprint_inputs))
        .route("/{blueprint_id}/outline", get(handlers::blueprint_outline))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_blueprint))
        .route(
            "/{blueprint_id}",
            patch(handlers::update_blueprint).delete(handlers::delete_blueprint),
        )
        .route("/{blueprint_id}/realms", post(handlers::expand_blueprint))
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
