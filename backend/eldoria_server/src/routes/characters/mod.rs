use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::post};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{
        PermissionMiddlewareState, SessionMiddlewareState, server_permission_middleware,
        session_csrf_middleware,
    },
};

pub(super) mod dto;
mod handlers;
pub(super) mod types;

pub(super) fn router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    let session_permission_routes = Router::new()
        .route("/import", post(handlers::import_card))
        .layer(axum::middleware::from_fn_with_state(
            permission_state.clone(),
            server_permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024));

    Router::new()
        .merge(session_permission_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
