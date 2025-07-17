use std::sync::Arc;

use axum::{extract::{Request, State}, http::{self, HeaderMap, StatusCode}, middleware::Next, response::Response};

use crate::{state::AppState, users::manager::{Role, User}};

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
    headers: HeaderMap,
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let user = state
        .user_manager
        .lock()
        .await
        .validate_session(token)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(user);
    Ok(next.run(request).await)
}

pub async fn auth(
    // run the `HeaderMap` extractor
    headers: HeaderMap,
    // you can also add more extractors here but the last
    // extractor must implement `FromRequest` which
    // `Request` does
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    match get_token(&headers) {
        Some(token) if token_is_valid(token) => {
            let response = next.run(request).await;
            Ok(response)
        }
        _ => {
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}

fn get_token(headers: &HeaderMap) -> Option<&str> {
    todo!()
}

fn token_is_valid(token: &str) -> bool {
    todo!()
}