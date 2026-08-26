mod dto;
mod handlers;

use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
pub(super) use dto::*;
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/secrets", get(handlers::list_secrets))
        .route("/profiles", get(handlers::list_profiles))
        .route("/profiles/active", get(handlers::get_active_profile))
        .route("/profiles/{profile_id}/models", get(handlers::list_models))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/secrets", post(handlers::create_secret))
        .route("/secrets/{secret_id}", put(handlers::rename_secret))
        .route("/secrets/{secret_id}", delete(handlers::delete_secret))
        .route("/profiles", post(handlers::create_profile))
        .route("/profiles/active", post(handlers::set_active_profile))
        .route("/profiles/{secret_id}", get(handlers::get_profile))
        .route("/profiles/{secret_id}", put(handlers::update_profile))
        .route("/profiles/{secret_id}", delete(handlers::delete_profile))
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
