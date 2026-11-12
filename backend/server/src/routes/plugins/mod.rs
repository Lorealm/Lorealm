mod handlers;

use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
    services::plugins::MAX_PACKAGE,
};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_plugins))
        .route("/marketplace", get(handlers::marketplace))
        .route(
            "/marketplace/{handle}/readme",
            get(handlers::marketplace_readme),
        )
        .route("/{handle}/logs", get(handlers::list_plugin_logs))
        .route("/{handle}/readme", get(handlers::plugin_readme))
        .route("/{handle}/settings", get(handlers::plugin_settings))
        .route("/{handle}/frontend/{entry}", get(handlers::frontend_entry))
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
        .route("/{handle}/reload", post(handlers::reload_plugin))
        .route(
            "/marketplace/{handle}/install",
            post(handlers::install_listed_plugin),
        )
        .route("/install", post(handlers::install_plugin))
        .route(
            "/install/upload",
            put(handlers::upload_plugin).layer(DefaultBodyLimit::max(MAX_PACKAGE)),
        )
        .route("/{handle}", delete(handlers::uninstall_plugin))
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
