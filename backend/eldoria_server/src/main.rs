use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use eldoria_storage::StorageConfig;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{config::HostingConfig, core::AppState, middleware::SessionMiddlewareState};

mod config;
mod core;
mod errors;
mod extract;
mod middleware;
mod routes;
mod services;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::new("DEBUG").add_directive("sqlx=warn".parse().unwrap()))
        .init();

    let storage_config = StorageConfig::server_default();
    let state = Arc::new(AppState::new(storage_config).await?);

    let session_service = Arc::clone(state.services().sessions());
    let middleware_state = SessionMiddlewareState::new(session_service, state.secret());

    // Extremely open for testing purposes
    // TODO: IMPORTANT: Make more restrictive
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest(
            "/api/v1",
            routes::api_routes(Arc::clone(&state), middleware_state).await,
        )
        .layer(cors);

    let cfg_host = state.config().get::<HostingConfig>();

    let addr = SocketAddr::from((cfg_host.ip, cfg_host.port));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();

    Ok(())
}
