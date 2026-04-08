use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware},
};

pub(super) mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/me", get(handlers::me))
        .route("/{user_id}", get(handlers::get_user))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));
    let public_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));

    Router::new()
        .merge(public_routes)
        .merge(session_only_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
