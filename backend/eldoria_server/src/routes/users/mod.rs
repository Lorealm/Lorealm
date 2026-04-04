use std::sync::Arc;

use axum::{Router, routing::post};
use tower_cookies::CookieManagerLayer;

use crate::{core::AppState, middleware::SessionMiddlewareState};

pub(super) mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let public_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));

    Router::new()
        .merge(public_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
