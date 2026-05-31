use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

mod dto;
mod handlers;

use dto::*;

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_lorebooks))
        .route("/{lorebook_id}", get(handlers::get_lorebook))
        .route(
            "/{lorebook_id}/entries/{entry_id}",
            get(handlers::get_entry),
        )
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_lorebook))
        .route("/{lorebook_id}", put(handlers::update_lorebook))
        .route("/{lorebook_id}", delete(handlers::delete_lorebook))
        .route("/{lorebook_id}/entries", post(handlers::create_entry))
        .route(
            "/{lorebook_id}/entries/{entry_id}",
            put(handlers::update_entry),
        )
        .route(
            "/{lorebook_id}/entries/{entry_id}",
            delete(handlers::delete_entry),
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
