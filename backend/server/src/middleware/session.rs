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
    users::user_session::UserState,
};

pub async fn get_user_session(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // `auth_middleware` must have already inserted the authenticated user
    let user = request
        .extensions()
        .get::<User>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract the token string from the cookie header
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

    // Now work with the in-memory session manager
    let mut sm = state.state_manager.write().await;

    // Either get existing session or insert a new one for this token
    let user_session = if let Some(session) = sm.get_session(&token) {
        session.clone()
    } else {
        let session = UserState::new(
            user.id.clone(),    // assuming User has `id: String`
            token.clone()
        );
        sm.insert_session(token.clone(), session.clone());
        session
    };

    // Attach session to request for downstream extractors
    request.extensions_mut().insert(user_session);

    Ok(next.run(request).await)
}
