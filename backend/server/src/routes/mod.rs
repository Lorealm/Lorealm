use std::sync::Arc;

use axum::Router;

use crate::{
    core::AppState,
    middleware::SessionMiddlewareState,
};

mod blobs;
mod blueprints;
mod characters;
mod chat;
mod config;
mod connection;
mod lorebooks;
mod persona;
mod plugins;
mod realm;
mod roles;
mod templates;
mod tokenizers;
mod tools;
mod users;
mod widgets;

pub use realm::{
    CreateAgentRequest, CreateScenarioRequest, PlayerTurnRequest, RunTurnRequest,
    UpdateRealmRequest,
};

pub async fn api_routes(
    app_state: Arc<AppState>,
    session_state: SessionMiddlewareState,
) -> Router {
    Router::new()
        .nest(
            "/blobs",
            blobs::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/users",
            users::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/characters",
            characters::router(Arc::clone(&app_state), session_state.clone()),
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
        .nest(
            "/widgets",
            widgets::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/plugins",
            plugins::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/tokenizers",
            tokenizers::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/roles",
            roles::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/config",
            config::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/realms",
            realm::router(Arc::clone(&app_state), session_state.clone()),
        )
        .nest(
            "/blueprints",
            blueprints::router(Arc::clone(&app_state), session_state.clone()),
        )
}
