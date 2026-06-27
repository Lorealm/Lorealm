use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{
        PermissionMiddlewareState, SessionMiddlewareState, server_permission_middleware,
        session_auth_middleware, session_csrf_middleware,
    },
};

mod handlers;

pub(super) fn router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::get_full_config))
        .route("/{key}", get(handlers::get_config))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let session_permission_routes = Router::new()
        .route("/{key}", patch(handlers::patch_config))
        .layer(axum::middleware::from_fn_with_state(
            permission_state.clone(),
            server_permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ));

    Router::new()
        .merge(session_only_routes)
        .merge(session_permission_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
