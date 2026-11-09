use std::sync::Arc;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post, put},
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
        .route("/me", get(handlers::me))
        .route("/me/sessions", get(handlers::list_sessions))
        .route("/me/permissions", get(handlers::my_permissions))
        .route("/{user_id}", get(handlers::get_user))
        .route("/", get(handlers::list_users))
        .route("/invites", get(handlers::list_invites))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/logout", post(handlers::logout))
        .route("/me", patch(handlers::update_profile))
        .route("/me/password", put(handlers::change_password))
        .route("/me/avatar", put(handlers::put_avatar))
        .route("/me/sessions", delete(handlers::revoke_other_sessions))
        .route(
            "/me/sessions/{session_id}",
            delete(handlers::revoke_session),
        )
        .route("/", post(handlers::create_user))
        .route("/{user_id}", patch(handlers::admin_update_user))
        .route("/{user_id}", delete(handlers::delete_user))
        .route(
            "/{user_id}/sessions",
            delete(handlers::revoke_user_sessions),
        )
        .route("/invites", post(handlers::create_invite))
        .route("/invites/{invite_id}", delete(handlers::delete_invite))
        .layer(DefaultBodyLimit::max(20 * 1024 * 1024))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ));

    let public_routes = Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));

    Router::new()
        .merge(public_routes)
        .merge(session_only_routes)
        .merge(csrf_routes)
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
