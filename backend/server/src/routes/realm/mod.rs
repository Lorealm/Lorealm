use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_realms))
        .route("/{realm_id}", get(handlers::get_realm))
        .route("/{realm_id}/versions", get(handlers::list_versions))
        .route(
            "/{realm_id}/versions/{version_id}",
            get(handlers::get_version),
        )
        .route("/{realm_id}/resources", get(handlers::list_resources))
        .route("/{realm_id}/export", get(handlers::export_realm))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/import", post(handlers::import_realm))
        .route("/{realm_id}", delete(handlers::delete_realm))
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
