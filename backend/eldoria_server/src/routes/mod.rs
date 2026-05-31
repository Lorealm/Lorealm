use std::sync::Arc;

use axum::Router;

use crate::{
    core::AppState,
    middleware::{PermissionMiddlewareState, SessionMiddlewareState},
};

mod characters;
mod lorebooks;
mod templates;
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
}
