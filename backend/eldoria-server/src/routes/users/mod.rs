use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    app_state::AppState,
    middleware::authentication::{
        SessionMiddlewareState, session_auth_middleware, session_csrf_middleware,
    },
};

mod auth;

pub(super) fn auth_router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
) -> axum::Router {
    // Routes that only need session auth (GET requests)
    let session_only_routes = Router::new()
        .route("/me", get(auth::me))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    // Routes that need both session and CSRF (POST/PUT/DELETE)
    let csrf_routes = Router::new()
        .route("/me", post(auth::update_profile))
        .route("/logout", post(auth::logout))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ));

    // Public routes (no auth)
    let public_routes = Router::new().route("/login", post(auth::login));

    // Merge all routes
    Router::new()
        .merge(public_routes)
        .merge(session_only_routes)
        .merge(csrf_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
