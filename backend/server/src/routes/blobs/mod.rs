use std::sync::Arc;

use axum::{Router, routing::get};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware},
};

mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    Router::new()
        .route("/{hash}", get(handlers::get_blob))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ))
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
