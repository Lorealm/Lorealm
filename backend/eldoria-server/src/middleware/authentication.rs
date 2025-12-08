use axum::{
    extract::{FromRequestParts, Request, State},
    http::{HeaderMap, StatusCode, header, request::Parts},
    middleware::Next,
    response::Response,
};
use tower_cookies::Cookies;
use uuid::Uuid;

use crate::{
    app_state::StorageProvider,
    models::{AuthError, UserDevice},
    services::authentication::{Session, error::SessionError},
};

/// Extension that gets added to requests after successful session validation
#[derive(Clone)]
pub struct AuthenticatedSession {
    pub session: Session,
    pub user_id: Uuid,
}

/// Shared state for the middleware
#[derive(Clone)]
pub struct SessionMiddlewareState {
    pub storage_provider: StorageProvider,
    pub server_secret: String,
    pub session_cookie_name: String,
    pub csrf_header_name: String,
}

impl SessionMiddlewareState {
    pub fn new(storage_provider: StorageProvider, server_secret: String) -> Self {
        Self {
            storage_provider,
            server_secret,
            session_cookie_name: "session_token".to_string(),
            csrf_header_name: "X-CSRF-Token".to_string(),
        }
    }

    pub fn with_cookie_name(mut self, name: String) -> Self {
        self.session_cookie_name = name;
        self
    }

    pub fn with_csrf_header(mut self, name: String) -> Self {
        self.csrf_header_name = name;
        self
    }
}

/// Middleware that validates sessions but does NOT validate CSRF
/// Use this for GET requests or endpoints that don't need CSRF protection
pub async fn session_auth_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let start = std::time::Instant::now();
    let session_service = state.storage_provider.session_service();

    // Extract session token from cookie
    let session_token = cookies
        .get(&state.session_cookie_name)
        .ok_or(AuthError::Session(SessionError::MissingToken))?
        .value()
        .to_string();

    tracing::debug!("session_auth: extracting user device");

    // Build user device from request
    let user_device = extract_user_device(&headers)?;
    println!("user device extracted:");

    // Retrieve and validate session
    let mut session =
        Session::retrieve(&session_token, &state.server_secret, &session_service).await?;

    // Validate session (expiry, revocation, device match)
    session.validate(&user_device).await?;

    // Bump session activity (implements sliding expiration)
    session.bump(&session_service).await?;

    // Add authenticated session to request extensions
    request.extensions_mut().insert(AuthenticatedSession {
        session: session.clone(),
        user_id: session.user_id(),
    });

    let elapsed = start.elapsed();
    tracing::debug!(
        "session_auth: completed in {:?} (user_id: {})",
        elapsed,
        session.user_id()
    );

    Ok(next.run(request).await)
}

/// Middleware that validates BOTH session AND CSRF token
/// Use this for state-changing requests (POST, PUT, DELETE, PATCH)
pub async fn session_csrf_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let start = std::time::Instant::now();
    let session_service = state.storage_provider.session_service();

    // Extract session token from cookie
    let session_token = cookies
        .get(&state.session_cookie_name)
        .ok_or(AuthError::Session(SessionError::MissingToken))?
        .value()
        .to_string();

    // Extract CSRF token from header
    let csrf_token = headers
        .get(&state.csrf_header_name)
        .and_then(|h| h.to_str().ok())
        .ok_or(SessionError::MissingCsrfToken)?
        .to_string();

    // Build user device from request
    let user_device = extract_user_device(&headers)?;

    // Retrieve and validate session
    let mut session =
        Session::retrieve(&session_token, &state.server_secret, &session_service).await?;

    // Validate session (expiry, revocation, device match)
    session.validate(&user_device).await?;

    // Validate CSRF token
    session.verify_csrf_token(&csrf_token, &state.server_secret)?;

    // Bump session activity
    session.bump(&session_service).await?;

    // Check if we should rotate CSRF
    let new_csrf_token = if session.should_rotate_csrf() {
        tracing::debug!("session_csrf: rotating CSRF token");
        Some(
            session
                .rotate_csrf(&state.server_secret, &session_service)
                .await?,
        )
    } else {
        None
    };

    // Add authenticated session to request extensions
    request.extensions_mut().insert(AuthenticatedSession {
        session: session.clone(),
        user_id: session.user_id(),
    });

    // If we rotated CSRF, add it to response extensions so handlers can access it
    if let Some(new_token) = new_csrf_token.clone() {
        request.extensions_mut().insert(NewCsrfToken(new_token));
    }

    let elapsed = start.elapsed();
    tracing::debug!(
        "session_csrf: completed in {:?} (user_id: {}, csrf_rotated: {})",
        elapsed,
        session.user_id(),
        new_csrf_token.is_some()
    );

    Ok(next.run(request).await)
}

/// Extension that contains a new CSRF token after rotation
#[derive(Clone)]
pub struct NewCsrfToken(pub String);

/// Helper to extract user device info from request headers
fn extract_user_device(headers: &HeaderMap) -> Result<UserDevice, AuthError> {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("Unknown")
        .to_string();

    // Try to get real IP from common proxy headers
    let ip = headers
        .get("X-Forwarded-For")
        .or_else(|| headers.get("X-Real-IP"))
        .or_else(|| headers.get("CF-Connecting-IP")) // Cloudflare
        .and_then(|h| h.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string());

    Ok(UserDevice { ip, user_agent })
}

impl<S> FromRequestParts<S> for AuthenticatedSession
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl std::future::Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            parts
                .extensions
                .get::<AuthenticatedSession>()
                .cloned()
                .ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Session not found in request extensions. Did you forget to add the middleware?",
                ))
        }
    }
}
