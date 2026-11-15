use std::{
    future::IntoFuture,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Context;
use axum::{
    Router,
    http::{HeaderValue, header},
};
use tokio_util::sync::CancellationToken;
use tower_http::{
    services::{ServeDir, ServeFile},
    set_header::SetResponseHeaderLayer,
};
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::{HostingConfig, TaskConfig},
    core::AppState,
    middleware::SessionMiddlewareState,
    setup::run_setup,
};

mod config;
mod core;
mod errors;
mod extract;
mod frontend;
mod middleware;
mod routes;
mod services;
mod setup;
#[cfg(test)]
mod tests;
mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenv::dotenv();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
                .add_directive("sqlx=warn".parse().unwrap())
                .add_directive("h2=warn".parse().unwrap())
                .add_directive("hyper_util=warn".parse().unwrap())
                .add_directive("reqwest=warn".parse().unwrap())
                .add_directive("wasmtime=warn".parse().unwrap())
                .add_directive("cranelift=warn".parse().unwrap()),
        )
        .init();

    let state = Arc::new(AppState::new().await?);

    run_setup(Arc::clone(&state)).await?;

    let cfg_host = state.config().get::<HostingConfig>();
    let cfg_task = state.config().get::<TaskConfig>();

    let session_service = Arc::clone(state.services().sessions());
    let session_state =
        SessionMiddlewareState::new(session_service, state.secret(), cfg_host.cookie_secure);

    let app = Router::new().nest(
        "/api/v1",
        routes::api_routes(Arc::clone(&state), session_state).await,
    );

    let cleanup = state
        .services()
        .start_cleanup_task(Duration::from_secs(cfg_task.cleanup_interval));

    let serve_frontend = cfg_host.serve_frontend
        && match frontend::ensure_frontend(&cfg_host.frontend_dir).await {
            Ok(()) => true,
            Err(e) => {
                tracing::warn!("serving the API only, the web UI is unavailable: {e:#}");
                false
            }
        };

    let app = if serve_frontend {
        let dir = cfg_host.frontend_dir.canonicalize().with_context(|| {
            format!(
                "serve_frontend is on but {:?} does not exist (cwd {:?})",
                cfg_host.frontend_dir,
                std::env::current_dir().unwrap()
            )
        })?;
        tracing::debug!("serving frontend from {}", dir.display());

        let assets = Router::new()
            .fallback_service(ServeDir::new(dir.join("assets")))
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            ));

        let shell = Router::new()
            .fallback_service(ServeDir::new(&dir).fallback(ServeFile::new(dir.join("index.html"))))
            .layer(SetResponseHeaderLayer::overriding(
                header::CACHE_CONTROL,
                HeaderValue::from_static("no-cache"),
            ));

        app.nest("/assets", assets).fallback_service(shell)
    } else {
        app
    };

    let ip: IpAddr = cfg_host.ip.parse()?;
    if !cfg_host.cookie_secure && !ip.is_loopback() {
        tracing::warn!(
            "hosting.cookie_secure is off and the server is bound to {ip}: session cookies \
             will travel unencrypted. This is fine on a trusted LAN, but not for public exposure."
        );
    }
    let addr = SocketAddr::from((ip, cfg_host.port));
    tracing::info!("Server starting on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("could not bind to {addr}; is the port already in use?"))?;
    let stop = CancellationToken::new();
    let server = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(stop.clone().cancelled_owned())
    .into_future();
    let mut server = std::pin::pin!(server);

    tokio::select! {
        res = &mut server => {
            res.context("server stopped unexpectedly")?;
            anyhow::bail!("server stopped unexpectedly");
        }
        () = shutdown_signal() => {}
    }

    tracing::info!(
        "Shutting down: finishing in-flight requests (up to {}s)",
        SHUTDOWN_DRAIN.as_secs()
    );
    stop.cancel();
    match tokio::time::timeout(SHUTDOWN_DRAIN, server).await {
        Ok(res) => res.context("server failed while shutting down")?,
        Err(_) => tracing::warn!("requests still open after the drain period, closing them"),
    }

    cleanup.abort();
    state.services().plugins().shutdown().await;
    tracing::info!("Shutdown complete");

    Ok(())
}

/// How long in-flight requests get to finish once shutdown starts.
const SHUTDOWN_DRAIN: Duration = Duration::from_secs(5);

/// Resolves on SIGTERM or Ctrl+C.
async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(e) = tokio::signal::ctrl_c().await {
            tracing::error!("could not listen for Ctrl+C: {e}");
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut sig) => {
                sig.recv().await;
            }
            Err(e) => {
                tracing::error!("could not listen for SIGTERM: {e}");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => tracing::info!("received Ctrl+C"),
        () = terminate => tracing::info!("received SIGTERM"),
    }
}
