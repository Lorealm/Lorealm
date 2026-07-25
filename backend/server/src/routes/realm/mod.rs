use std::sync::Arc;

use axum::{
    Router,
    routing::{get, patch, post, put},
};
use tower_cookies::CookieManagerLayer;

use crate::{
    core::AppState,
    middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware},
};

pub mod dto;
mod handlers;

pub use dto::{
    CreateAgentRequest, CreateScenarioRequest, PlayerTurnRequest, RunTurnRequest,
    UpdateRealmRequest,
};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_realms))
        .route("/{realm_id}", get(handlers::get_realm))
        .route("/{realm_id}/config", get(handlers::get_config))
        .route("/{realm_id}/versions", get(handlers::list_versions))
        .route(
            "/{realm_id}/versions/{version_id}",
            get(handlers::get_version),
        )
        .route("/{realm_id}/agents", get(handlers::list_agents))
        .route(
            "/{realm_id}/agents/{agent_id}/grants",
            get(handlers::get_grants),
        )
        .route(
            "/{realm_id}/agents/{agent_id}/actions",
            get(handlers::list_actions),
        )
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_realm))
        .route(
            "/{realm_id}",
            patch(handlers::update_realm).delete(handlers::delete_realm),
        )
        .route("/{realm_id}/config", put(handlers::set_config))
        .route(
            "/{realm_id}/versions/{version_id}/restore",
            post(handlers::restore_version),
        )
        .route("/{realm_id}/agents", post(handlers::create_agent))
        .route("/{realm_id}/agents/order", put(handlers::reorder_agents))
        .route(
            "/{realm_id}/agents/{agent_id}",
            put(handlers::update_agent).delete(handlers::delete_agent),
        )
        .route(
            "/{realm_id}/agents/{agent_id}/grants",
            put(handlers::set_grants),
        )
        .route("/{realm_id}/imports", put(handlers::set_imports))
        .route("/{realm_id}/scenarios", post(handlers::create_scenario))
        .route(
            "/{realm_id}/scenarios/{scenario_id}",
            put(handlers::update_scenario).delete(handlers::delete_scenario),
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
