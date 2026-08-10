use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

mod dto;
mod handlers;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_personas))
        .route("/active", get(handlers::get_active_persona))
        .route("/overrides", get(handlers::list_overrides))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    // `POST /` to create and `PUT /active` to select, matching lorebooks,
    // templates and tokenizers.
    let csrf_routes = Router::new()
        .route("/", post(handlers::create_persona))
        .route("/active", put(handlers::set_active_persona))
        .route("/overrides/{character_id}", put(handlers::set_override))
        .route(
            "/overrides/{character_id}",
            delete(handlers::clear_override),
        )
        .route("/{persona_id}", put(handlers::save_persona))
        .route("/{persona_id}/avatar", get(handlers::get_avatar))
        .route("/{persona_id}/avatar", put(handlers::put_avatar))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024))
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
