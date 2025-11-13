use std::sync::Arc;

use axum::{Router, routing::get};
use secrecy::ExposeSecret;

use crate::{app_state::AppState, middleware::authentication::SessionMiddlewareState, routes::user_router};

pub async fn api_routes(app_state: Arc<AppState>) -> Router {
    let storage_lock = app_state.storage();
    let storage_provider = storage_lock.read().await.clone(); // Clone the provider
    
    let middleware_state = SessionMiddlewareState::new(
        storage_provider,
        app_state.server_secret().expose_secret().to_string(),
    );
    
    drop(storage_lock);
    
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .nest("/user", user_router(app_state, middleware_state))
}
