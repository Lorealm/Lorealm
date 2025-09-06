use std::sync::Arc;

use axum::{debug_middleware, extract::{Request, State}, http::{HeaderMap, StatusCode}, middleware::Next, response::Response};
use axum_extra::extract::CookieJar;

use crate::{routes::csrf::get_csrf_token, state::AppState, users::manager::{Role, User}};

// Middleware for route protection
pub fn require_auth(user: &User, required_role: Option<Role>) -> Result<(), &'static str> {
    if let Some(role) = required_role {
        let user_role = Role::from_str(&user.role)?;
        if user_role != Role::Admin && user_role != role {
            return Err("Insufficient permissions");
        }
    }
    Ok(())
}

// Middleware to extract user from session token
#[debug_middleware]
pub async fn auth_middleware(
    cookies: CookieJar,
    State(state): axum::extract::State<Arc<AppState>>,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("auth middleware");
    let token = cookies
        .get("auth_token")
        .map(|cookie| cookie.value().to_string())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    log::info!("token: {}", &token);
    let manager = state.user_manager.clone();
    let mut mgr_write = manager.write().await;

    // Try validating the session
    let user = match mgr_write.validate_session(&token).await {
        Ok(user) => user,
        Err(_) => {
            // Attempt auto-migration of session token
            match mgr_write.migrate_token(&token).await {
                Ok(new_token) => {
                    // Validate the new token
                    match mgr_write.validate_session(&new_token).await {
                        Ok(user) => user,
                        Err(_) => return Err(StatusCode::UNAUTHORIZED),
                    }
                }
                Err(_) => return Err(StatusCode::UNAUTHORIZED),
            }
        }
    };

    log::info!("user: {:#?}", user);

    // Insert the user into the request extensions
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn verify_csrf(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("---- verify csrf middleware ----");
    let token = cookies.get("auth_token").and_then(|cookie| Some(cookie.value().to_string()));
    if token.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    log::info!("token retrieved");

    let csrf_token = get_csrf_token(state, token.unwrap()).await.map_err(|_| StatusCode::UNAUTHORIZED)?;
    let client_token = headers.get("X-CSRF-TOKEN").and_then(|h| Some(h.to_str().unwrap().to_string()));

    if client_token.is_none() {
        log::warn!("client token not found");
        log::info!("---- verify csrf middleware ----");
        return Err(StatusCode::UNAUTHORIZED);
    }

    log::info!("server token: {}; client token: {}", csrf_token, &client_token.clone().unwrap());
    if client_token.unwrap() != csrf_token {
        log::warn!("csrf token mismatch");
        log::info!("---- verify csrf middleware ----");
        return Err(StatusCode::UNAUTHORIZED);
    }

    log::info!("csrf validated");
    log::info!("---- verify csrf middleware ----");

    Ok(next.run(request).await)
    
}