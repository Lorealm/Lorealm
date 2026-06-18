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
        .route("/", get(handlers::list_conversations))
        .route("/{conversation_id}", get(handlers::get_conversation))
        .route("/{conversation_id}/messages", get(handlers::list_messages))
        .route(
            "/{conversation_id}/messages/{message_id}",
            get(handlers::get_message),
        )
        .route(
            "/{conversation_id}/messages/{message_id}/state",
            get(handlers::get_message_state),
        )
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::start_chat))
        .route("/{conversation_id}", delete(handlers::delete_conversation))
        .route("/{conversation_id}/messages", put(handlers::edit_message))
        .route(
            "/{conversation_id}/messages/{message_id}",
            delete(handlers::delete_message),
        )
        .route(
            "/{conversation_id}/completions",
            post(handlers::completions).delete(handlers::cancel_completion),
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
