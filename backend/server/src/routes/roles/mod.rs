use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch, post},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub(super) mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_roles))
        .route("/{role_id}/members", get(handlers::list_members))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_role))
        .route("/{role_id}", patch(handlers::update_role))
        .route("/{role_id}", delete(handlers::delete_role))
        .route("/{role_id}/members", post(handlers::add_member))
        .route(
            "/{role_id}/members/{user_id}",
            delete(handlers::remove_member),
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
