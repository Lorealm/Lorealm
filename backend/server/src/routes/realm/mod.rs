use std::sync::Arc;

use axum::{Router, routing::{delete, get, post, put}};
use tower_cookies::CookieManagerLayer;

use crate::{core::AppState, middleware::{SessionMiddlewareState, session_auth_middleware, session_csrf_middleware}};

pub mod dto;
mod handlers;

pub use dto::{CreateAgentRequest, PlayerTurnRequest};

pub(super) fn router(app_state: Arc<AppState>, middleware_state: SessionMiddlewareState) -> Router {
    let session_only_routes = Router::new()
        .route("/", get(handlers::list_realms))
        .route("/{realm_id}", get(handlers::get_realm))
        .route("/{realm_id}/agents", get(handlers::list_agents))
        .route(
            "/{realm_id}/agents/{agent_id}/grants",
            get(handlers::get_grants),
        )
        .route(
            "/{realm_id}/agents/{agent_id}/observations",
            get(handlers::list_observations),
        )
        .route(
            "/{realm_id}/agents/{agent_id}/actions",
            get(handlers::list_actions),
        )
        .route(
            "/{realm_id}/agents/{agent_id}/state",
            get(handlers::agent_state),
        )
        .route("/{realm_id}/events", get(handlers::list_events))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_auth_middleware,
        ));

    let csrf_routes = Router::new()
        .route("/", post(handlers::create_realm))
        .route("/{realm_id}", delete(handlers::delete_realm))
        .route("/{realm_id}/agents", post(handlers::create_agent))
        .route(
            "/{realm_id}/agents/{agent_id}/grants",
            put(handlers::set_grants),
        )
        .route(
            "/{realm_id}/tick",
            post(handlers::tick).delete(handlers::cancel_tick),
        )
        .route("/{realm_id}/turn", post(handlers::player_turn))
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
