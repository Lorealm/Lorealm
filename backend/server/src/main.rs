use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::{HostingConfig, TaskConfig},
    core::AppState,
    middleware::{PermissionMiddlewareState, SessionMiddlewareState},
    setup::run_setup,
};

mod config;
mod core;
mod errors;
mod extract;
mod middleware;
mod routes;
mod services;
mod setup;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::new("DEBUG")
                .add_directive("sqlx=warn".parse().unwrap())
                .add_directive("h2=warn".parse().unwrap())
                .add_directive("hyper_util=warn".parse().unwrap())
                .add_directive("reqwest=warn".parse().unwrap()),
        )
        .init();

    let state = Arc::new(AppState::new().await?);

    run_setup(Arc::clone(&state)).await?;

    let session_service = Arc::clone(state.services().sessions());
    let permission_service = Arc::clone(state.services().permissions());
    let session_state = SessionMiddlewareState::new(session_service, state.secret());
    let permission_state = PermissionMiddlewareState::new(permission_service);

    // Extremely open for testing purposes
    // TODO: IMPORTANT: Make more restrictive
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .nest(
            "/api/v1",
            routes::api_routes(Arc::clone(&state), session_state, permission_state).await,
        )
        .layer(cors);

    let cfg_host = state.config().get::<HostingConfig>();
    let cfg_task = state.config().get::<TaskConfig>();

    let _cleanup = state
        .services()
        .start_cleanup_task(Duration::from_secs(cfg_task.cleanup_interval));

    let ip: IpAddr = cfg_host.ip.parse()?;
    let addr = SocketAddr::from((ip, cfg_host.port));
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
