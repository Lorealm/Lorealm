use std::sync::Arc;

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    middleware::Next,
    response::Response,
};
use secrecy::SecretString;
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time},
};

use crate::{
    errors::AuthError,
    extract::AuthenticatedSession,
    services::session::{SessionService, ValidatedSession},
};

// ─── Middleware State ────────────────────────────────────────

#[derive(Clone)]
pub struct SessionMiddlewareState {
    pub session_service: Arc<SessionService>,
    pub server_secret: SecretString,
    pub session_cookie_name: String,
    pub csrf_cookie_name: String,
    pub csrf_header_name: String,
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
}

impl SessionMiddlewareState {
    pub fn new(session_service: Arc<SessionService>, server_secret: SecretString) -> Self {
        Self {
            session_service,
            server_secret,
            session_cookie_name: "session_token".to_string(),
            csrf_cookie_name: "csrf_token".to_string(),
            csrf_header_name: "X-CSRF-Token".to_string(),
            cookie_domain: None,
            // In development mode we use non-secure cookies. Release mode enforces secure cookies.
            cookie_secure: !cfg!(debug_assertions),
        }
    }

    pub fn with_session_cookie_name(mut self, name: impl Into<String>) -> Self {
        self.session_cookie_name = name.into();
        self
    }

    pub fn with_csrf_header(mut self, name: impl Into<String>) -> Self {
        self.csrf_header_name = name.into();
        self
    }

    pub fn with_cookie_domain(mut self, domain: impl Into<String>) -> Self {
        self.cookie_domain = Some(domain.into());
        self
    }

    pub fn with_cookie_secure(mut self, secure: bool) -> Self {
        self.cookie_secure = secure;
        self
    }

    /// Build a session cookie (HttpOnly, not accessible to JS)
    fn build_session_cookie(&self, value: &str, max_age_secs: i64) -> Cookie<'static> {
        let mut cookie = Cookie::build((self.session_cookie_name.clone(), value.to_string()))
            .path("/")
            .http_only(true)
            .secure(self.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::seconds(max_age_secs))
            .build();

        if let Some(ref domain) = self.cookie_domain {
            cookie.set_domain(domain.clone());
        }

        cookie
    }

    fn build_csrf_cookie(&self, value: &str, max_age_secs: i64) -> Cookie<'static> {
        let mut cookie = Cookie::build((self.csrf_cookie_name.clone(), value.to_string()))
            .path("/")
            .http_only(false)
            .secure(self.cookie_secure)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::seconds(max_age_secs))
            .build();

        if let Some(ref domain) = self.cookie_domain {
            cookie.set_domain(domain.clone());
        }

        cookie
    }
}

enum TokenSource {
    Cookie,
    Bearer,
}

fn extract_session_token(
    cookies: &Cookies,
    headers: &HeaderMap,
    cookie_name: &str,
) -> Result<(String, TokenSource), AuthError> {
    // Cookie takes priority (browser context)
    if let Some(cookie) = cookies.get(cookie_name) {
        return Ok((cookie.value().to_string(), TokenSource::Cookie));
    }

    // Fall back to Authorization: Bearer <token>
    if let Some(auth_header) = headers.get(header::AUTHORIZATION) {
        let value = auth_header.to_str().map_err(|_| AuthError::MissingToken)?;
        if let Some(token) = value.strip_prefix("Bearer ") {
            return Ok((token.to_string(), TokenSource::Bearer));
        }
    }

    Err(AuthError::MissingToken)
}

// ─── Session-only middleware (GET routes) ────────────────────

pub async fn session_auth_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let (session_token, _source) =
        extract_session_token(&cookies, &headers, &state.session_cookie_name)?;

    let validated = state
        .session_service
        .validate_session(&session_token, &state.server_secret)
        .await?;

    if let Some(ref new_token) = validated.new_token {
        let max_age = remaining_seconds(&validated);
        cookies.add(state.build_session_cookie(new_token, max_age));
    }

    request.extensions_mut().insert(AuthenticatedSession {
        session_id: validated.session.id,
        user_id: validated.session.user_id,
    });

    Ok(next.run(request).await)
}

// ─── Session + CSRF middleware (state-changing routes) ───────

pub async fn session_csrf_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let (session_token, source) =
        extract_session_token(&cookies, &headers, &state.session_cookie_name)?;

    let validated = state
        .session_service
        .validate_session(&session_token, &state.server_secret)
        .await?;

    // CSRF only needed for cookie-based auth (browser context)
    if matches!(source, TokenSource::Cookie) {
        let csrf_token = headers
            .get(&state.csrf_header_name)
            .and_then(|h| h.to_str().ok())
            .ok_or(AuthError::MissingCsrfToken)?
            .to_string();

        state
            .session_service
            .verify_csrf(&validated.session, &csrf_token, &state.server_secret)?;

        let new_csrf = state
            .session_service
            .maybe_rotate_csrf(&validated.session, &state.server_secret)
            .await?;

        if let Some(ref new_csrf_token) = new_csrf {
            let max_age = remaining_seconds(&validated);
            cookies.add(state.build_csrf_cookie(new_csrf_token, max_age));
        }
    }

    let max_age = remaining_seconds(&validated);
    if let Some(ref new_token) = validated.new_token {
        cookies.add(state.build_session_cookie(new_token, max_age));
    }

    request.extensions_mut().insert(AuthenticatedSession {
        session_id: validated.session.id,
        user_id: validated.session.user_id,
    });

    Ok(next.run(request).await)
}

// ─── Extractor ───────────────────────────────────────────────

impl<S> FromRequestParts<S> for AuthenticatedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedSession>()
            .cloned()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Not authenticated. Missing session middleware.",
            ))
    }
}

// ─── Helpers ─────────────────────────────────────────────────

fn remaining_seconds(validated: &ValidatedSession) -> i64 {
    (validated.session.expires_at - chrono::Utc::now())
        .num_seconds()
        .max(0)
}
