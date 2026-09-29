mod handlers;

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{
        PermissionMiddlewareState, SessionMiddlewareState, server_permission_middleware,
        session_auth_middleware, session_csrf_middleware,
    },
};

pub(super) fn router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_plugins))
        .route("/{handle}/logs", get(handlers::list_plugin_logs))
        .route("/{handle}/readme", get(handlers::plugin_readme))
        .route("/{handle}/settings", get(handlers::plugin_settings))
        .route("/{handle}/frontend/{entry}", get(handlers::frontend_entry))
        // Unsafe slots are only listed and served to users allowed them.
        .layer(axum::middleware::from_fn_with_state(
            permission_state.clone(),
            server_permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", put(handlers::set_plugin_order))
        .route("/{handle}", patch(handlers::toggle_plugin_enabled))
        .route("/{handle}/call", post(handlers::call_plugin))
        .route("/events", post(handlers::frontend_events))
        .route("/{handle}/settings", put(handlers::save_plugin_settings))
        .route("/{handle}/update", post(handlers::update_plugin))
        .route("/install", post(handlers::install_plugin))
        .route("/{handle}", delete(handlers::uninstall_plugin))
        .layer(axum::middleware::from_fn_with_state(
            permission_state,
            server_permission_middleware,
        ))
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
