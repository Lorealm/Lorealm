use std::sync::Arc;

use axum::Router;

use crate::{
    core::AppState,
    middleware::{PermissionMiddlewareState, SessionMiddlewareState},
};

mod characters;
mod chat;
mod connection;
mod lorebooks;
mod persona;
mod templates;
mod tools;
mod users;

pub async fn api_routes(
    app_state: Arc<AppState>,
    session_state: SessionMiddlewareState,
    permission_state: PermissionMiddlewareState,
) -> Router {
    Router::new()
        .nest(
            "/users",
            users::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/characters",
            characters::router(
                Arc::clone(&app_state),
                session_state.clone(),
                permission_state.clone(),
            ),
        )
        .nest(
            "/templates",
            templates::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/lorebooks",
            lorebooks::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/chat",
            chat::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/persona",
            persona::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/connections",
            connection::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/tools",
            tools::router(Arc::clone(&app_state), session_state.clone()),
        )
}
