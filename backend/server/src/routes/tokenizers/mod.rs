use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    config::TokenizerConfig,
    core::AppState,
    middleware::{
        PermissionMiddlewareState, SessionMiddlewareState, server_permission_middleware,
        session_auth_middleware, session_csrf_middleware,
    },
};

pub(super) mod dto;
mod handlers;

pub(super) fn router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    let config = app_state.config().get::<TokenizerConfig>();
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_tokenizers))
        .route("/active", get(handlers::get_active_tokenizer))
        .route("/{tokenizer_id}", get(handlers::get_tokenizer))
        .route("/{tokenizer_id}/count", get(handlers::count))
        .route("/{tokenizer_id}/batchCount", get(handlers::batch_count))
        .route("/{tokenizer_id}/shares", get(handlers::list_shares))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let session_permission_routes = Router::new()
        .route("/active", put(handlers::set_active_tokenizer))
        .route("/active", delete(handlers::unset_active_tokenizer))
        .route("/import", post(handlers::import_tokenizer))
        .route("/network", post(handlers::create_from_network))
        .route("/{tokenizer_id}", delete(handlers::delete_tokenizer))
        .route("/{tokenizer_id}/shares", post(handlers::share_tokenizer))
        .route(
            "/{tokenizer_id}/shares/{user_id}",
            delete(handlers::unshare_tokenizer),
        )
        .layer(axum::middleware::from_fn_with_state(
            permission_state.clone(),
            server_permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ))
        .layer(DefaultBodyLimit::max(
            (config.max_tokenizer_size_mb * 1024 * 1024) as usize,
        ));

    Router::new()
        .merge(session_only_routes)
        .merge(session_permission_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
