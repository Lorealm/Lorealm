use std::sync::Arc;

use axum::{extract::{Request, State}, http::{self, HeaderMap, StatusCode}, middleware::Next, response::Response};
use axum_extra::extract::CookieJar;

use crate::{routes::csrf::{csrf_token, get_csrf_token}, state::AppState, users::manager::{Role, User}};

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
pub async fn auth_middleware(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    log::info!("auth middleware");
    let token = cookies.get("auth_token").and_then(|cookie| Some(cookie.value().to_string()));
    
    if token.is_none() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    log::info!("token: {}", &token.clone().unwrap());
    let user = state
        .user_manager
        .lock()
        .await
        .validate_session(&token.unwrap())
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    log::info!("user: {:#?}", user);
    
    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn verify_csrf(
    cookies: CookieJar,
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    mut request: Request,
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