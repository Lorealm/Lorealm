use axum::{
    Router,
    extract::{Json, Query, State},
    http::{StatusCode, header},
    middleware::from_fn_with_state,
    response::{
        Html, IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use axum_extra::extract::CookieJar;
use futures::{StreamExt, stream};
use llm_providers::{llm_provider::LLMProvider, openrouter::OpenRouterProvider, types::Message};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::HashMap, convert::Infallible, env, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tokio_stream::{Stream, wrappers::ReceiverStream};
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing::{Level, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod state;
use state::AppState;

use crate::{
    middleware::auth::{auth_middleware, verify_csrf},
    plugin_system::plugin_manager::PluginManager,
    routes::users::{admin_routes, auth_routes, protected_auth_routes, user_routes},
};

mod file;
mod llm;
pub mod middleware;
mod plugin_system;
mod resources;
mod routes;
pub mod systems;
pub mod settings;
mod users;
mod jwt;

// LLM-related types
#[derive(Serialize, Deserialize)]
struct StreamChunk {
    text: String,
    done: bool,
}

#[derive(Deserialize)]
struct StreamQueryParams {
    messages: String,
}

#[derive(Serialize, Deserialize)]
struct CompletionRequest {
    messages: Vec<Message>,
}

#[derive(Serialize)]
struct CompletionResponse {
    text: String,
}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize app state
    let app_state = Arc::new(AppState::new().await);
    app_state.initialize().await;
    initialize_plugin_manager(app_state.get_plugin_manager()).await;

    let tokens = app_state.tokenizers.lock().await.count_tokens(&"llama3".to_string(), &"I like big juice apples!".to_string()).await.unwrap();
    
    println!("Result (token count): {}", tokens);

    // Get current directory for static file serving
    let current_dir = std::env::current_dir().unwrap();
    info!("Current directory: {}", current_dir.display());

    // Determine static file path - check both 'dist' and 'static' directories
    let static_path = if current_dir.join("dist").exists() {
        current_dir.join("dist")
    } else if current_dir.join("static").exists() {
        current_dir.join("static")
    } else {
        // Default to static for production builds
        current_dir.join("static")
    };

    info!("Serving static files from: {}", static_path.display());

    // Build the application
    let app = create_app(app_state, static_path);

    // Get the port from environment or use default
    let port = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .expect("Invalid PORT");

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Starting server on http://{}", addr);

    // Run the server
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}

async fn initialize_plugin_manager(plugin_manager: Arc<Mutex<Option<PluginManager>>>) {
    async move {
        let mut guard = plugin_manager.lock().await;
        if let Some(plugin_manager) = guard.as_mut() {
            plugin_manager
                .initialize()
                .await
                .expect("Failed to initialize plugin manager");
        }
    }
    .await;
}

fn create_app(app_state: Arc<AppState>, static_path: PathBuf) -> Router {
    // Create static file service
    let static_files_service = ServeDir::new(&static_path)
        .precompressed_gzip()
        .append_index_html_on_directories(false);

    let api_routes = Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/status", get(status))
        // LLM endpoints
        .route("/stream", get(stream_handler))
        .route("/completion", post(completion_handler));

    // Main router

    Router::new()
        .nest("/api", api_routes)
        .nest("/api", auth_routes(app_state.as_ref().clone()))
        .nest(
            "/api/user",
            user_routes(app_state.as_ref().clone())
                .layer(from_fn_with_state(app_state.clone(), verify_csrf))
                .layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .nest(
            "/api/user",
            protected_auth_routes(app_state.as_ref().clone())
                .layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .nest(
            "/api/admin",
            admin_routes(app_state.as_ref().clone())
                .layer(from_fn_with_state(app_state.clone(), auth_middleware)),
        )
        .fallback_service(static_files_service.clone())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}

// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "eldoria-hub",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// Status endpoint with more detailed information
async fn status(cookies: CookieJar, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let auth_token = cookies
        .get("auth_token")
        .and_then(|cookie| Some(cookie.value().to_string()));
    let is_authenticated = if let Some(token) = auth_token {
        state
            .user_manager
            .lock()
            .await
            .validate_session(&token)
            .await
            .is_ok()
    } else {
        false
    };
    Json(json!({
        "status": "operational",
        "version": env!("CARGO_PKG_VERSION"),
        "plugins": {
            "count": state.get_plugin_manager().lock().await.as_ref().map(|pm| pm.get_plugin_count()).unwrap_or(0),
        },
        "authenticated": is_authenticated,
        "uptime": state.get_uptime().as_secs(),
    }))
}

// LLM stream handler
async fn stream_handler(
    Query(params): Query<StreamQueryParams>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let provider = Arc::new(OpenRouterProvider::new());

    let stream = async_stream::stream! {
        // Parse messages inside the stream
        let messages = match serde_json::from_str::<Vec<Message>>(&params.messages) {
            Ok(msgs) => msgs,
            Err(e) => {
                yield Ok(Event::default().data(serde_json::to_string(&StreamChunk {
                    text: format!("Invalid messages format: {}", e),
                    done: true,
                }).unwrap()));
                return;
            }
        };

        match provider.generate_stream(messages).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(response) => {
                            let text = response.text();
                            let is_done = response.json().get("done").is_some();

                            if !text.is_empty() || is_done {
                                yield Ok(Event::default().data(
                                    serde_json::to_string(&StreamChunk { text, done: is_done }).unwrap()
                                ));
                            }

                            if is_done {
                                break;
                            }
                        }
                        Err(e) => {
                            yield Ok(Event::default().data(
                                serde_json::to_string(&StreamChunk {
                                    text: format!("Error: {}", e),
                                    done: true,
                                }).unwrap()
                            ));
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                yield Ok(Event::default().data(
                    serde_json::to_string(&StreamChunk {
                        text: format!("Error: {}", e),
                        done: true,
                    }).unwrap()
                ));
            }
        }
    };

    Sse::new(stream)
}

// LLM completion handler
async fn completion_handler(Json(req): Json<CompletionRequest>) -> impl IntoResponse {
    let provider = Arc::new(OpenRouterProvider::new());

    match provider.generate(req.messages).await {
        Ok(response) => Json(CompletionResponse {
            text: response.text(),
        }),
        Err(e) => Json(CompletionResponse {
            text: format!("Error: {}", e),
        }),
    }
}

// Error handler
async fn handle_error(error: std::io::Error) -> impl IntoResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        format!("Internal server error: {}", error),
    )
}

