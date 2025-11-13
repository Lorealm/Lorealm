use std::sync::Arc;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    http::StatusCode,
};

use crate::{
    state::AppState,
    users::manager::User,
    users::user_state::UserStateManager,
};

/// Gets or creates a new session based on a User object inserted in an earlier step.
/// 
/// NOTE: Due to the `User` dependency, this must be run after 
/// [`crate::middleware::auth::auth_middleware`]
pub async fn user_state_middleware(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("state middleware");
    let user = request
        .extensions()
        .get::<User>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    log::info!("extracting token from headers");
    let token = request
        .headers()
        .get(axum::http::header::COOKIE)
        .and_then(|hv| hv.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header
                .split(';')
                .map(|s| s.trim())
                .find_map(|kv| {
                    let mut parts = kv.splitn(2, '=');
                    let k = parts.next()?;
                    let v = parts.next()?;
                    if k == "auth_token" {
                        Some(v.to_string())
                    } else {
                        None
                    }
                })
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;

    log::info!("retrieving session");
    let mut sm = state.state_manager.write().await;

    // Get or create the session
    let session_arc = if let Some(session) = sm.get_session(&token).cloned() {
        log::info!("session found");
        session
    } else {
        log::info!("creating new session");
        let session = UserStateManager::load(&state, &token).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sm.insert_session(token.clone(), session);
        sm.get_session(&token).unwrap().clone() // Safe because we just inserted it
    };

    // Insert the Arc<RwLock<UserState>> into the request extensions
    request.extensions_mut().insert(session_arc);

    Ok(next.run(request).await)
}