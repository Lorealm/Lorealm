use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    config::CharacterConfig,
    core::AppState,
    middleware::{
        PermissionMiddlewareState, SessionMiddlewareState, server_permission_middleware,
        session_auth_middleware, session_csrf_middleware,
    },
};

pub(super) mod dto;
mod handlers;
pub(super) mod types;

pub(super) fn router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    let config = app_state.config().get::<CharacterConfig>();
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_characters))
        .route("/", post(handlers::create_character))
        .route("/{character_id}", get(handlers::get_character_latest))
        .route("/{character_id}/image", get(handlers::get_image_for_latest))
        .route("/{character_id}/chats", get(handlers::list_character_chats))
        .route(
            "/{character_id}/versions",
            get(handlers::list_character_versions),
        )
        .route(
            "/{character_id}/versions/{version_id}",
            get(handlers::get_character_version),
        )
        .route(
            "/{character_id}/versions/{version_id}/image",
            get(handlers::get_image_for_version),
        )
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));
    let session_permission_routes = Router::new()
        .route("/import", put(handlers::import_card_blob))
        .route("/importBatch", post(handlers::import_card_multipart))
        .route("/{character_id}", put(handlers::edit_character))
        .route("/{character_id}/image", put(handlers::put_image))
        .layer(axum::middleware::from_fn_with_state(
            permission_state.clone(),
            server_permission_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ))
        .layer(DefaultBodyLimit::max(config.upload_limit_mb * 1024 * 1024));

    Router::new()
        .merge(session_only_routes)
        .merge(session_permission_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
