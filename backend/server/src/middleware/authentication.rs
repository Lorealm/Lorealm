use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, header},
    middleware::Next,
    response::Response,
};
use secrecy::SecretString;
use tower_cookies::{
    Cookie, Cookies,
    cookie::{SameSite, time},
};

use crate::{
    errors::{AppError, AuthError},
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
    pub fn new(
        session_service: Arc<SessionService>,
        server_secret: SecretString,
        cookie_secure: bool,
    ) -> Self {
        Self {
            session_service,
            server_secret,
            session_cookie_name: "session_token".to_string(),
            csrf_cookie_name: "csrf_token".to_string(),
            csrf_header_name: "X-CSRF-Token".to_string(),
            cookie_domain: None,
            cookie_secure,
        }
    }

    // pub fn with_session_cookie_name(mut self, name: impl Into<String>) -> Self {
    //     self.session_cookie_name = name.into();
    //     self
    // }

    // pub fn with_csrf_header(mut self, name: impl Into<String>) -> Self {
    //     self.csrf_header_name = name.into();
    //     self
    // }

    // pub fn with_cookie_domain(mut self, domain: impl Into<String>) -> Self {
    //     self.cookie_domain = Some(domain.into());
    //     self
    // }

    // pub fn with_cookie_secure(mut self, secure: bool) -> Self {
    //     self.cookie_secure = secure;
    //     self
    // }

    /// Build a session cookie
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

pub async fn session_auth_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let (session_token, source) =
        extract_session_token(&cookies, &headers, &state.session_cookie_name)?;

    let validated = state
        .session_service
        .validate_session(&session_token, &state.server_secret)
        .await?;

    let bearer_rotation = deliver_rotated_session_token(&state, &cookies, &validated, &source);

    request.extensions_mut().insert(AuthenticatedSession {
        session_id: validated.session.id,
        user_id: validated.session.user_id,
    });

    let mut response = next.run(request).await;
    attach_bearer_rotation(&mut response, bearer_rotation);
    Ok(response)
}

pub async fn session_csrf_middleware(
    State(state): State<SessionMiddlewareState>,
    cookies: Cookies,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
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

    let bearer_rotation = deliver_rotated_session_token(&state, &cookies, &validated, &source);

    request.extensions_mut().insert(AuthenticatedSession {
        session_id: validated.session.id,
        user_id: validated.session.user_id,
    });

    let mut response = next.run(request).await;
    attach_bearer_rotation(&mut response, bearer_rotation);
    Ok(response)
}

/// Hands a rotated session token back the way the client sent it: a cookie
/// is reissued directly, a Bearer token is returned so it can go in a header.
fn deliver_rotated_session_token(
    state: &SessionMiddlewareState,
    cookies: &Cookies,
    validated: &ValidatedSession,
    source: &TokenSource,
) -> Option<String> {
    let new_token = validated.new_token.as_ref()?;
    match source {
        TokenSource::Cookie => {
            let max_age = remaining_seconds(validated);
            cookies.add(state.build_session_cookie(new_token, max_age));
            None
        }
        TokenSource::Bearer => Some(new_token.clone()),
    }
}

/// Bearer clients can't receive cookies, so they get the new token as a header.
fn attach_bearer_rotation(response: &mut Response, token: Option<String>) {
    if let Some(token) = token
        && let Ok(value) = token.parse()
    {
        response.headers_mut().insert("X-New-Session-Token", value);
    }
}

fn remaining_seconds(validated: &ValidatedSession) -> i64 {
    (validated.session.expires_at - chrono::Utc::now())
        .num_seconds()
        .max(0)
}

#[cfg(test)]
mod tests {
    use axum::{Router, routing::post};
    use lorealm_storage::{backends::MemoryStorage, models::Snowflake};
    use tower_cookies::CookieManagerLayer;

    use super::*;
    use crate::{
        config::{SessionConfig, SlidingPolicy},
        extract::{DeviceType, UserDevice},
        services::snowflake::SnowflakeService,
    };

    /// A Bearer client on a CSRF-protected route must get its rotated token
    /// back as a header, not as a cookie it will never read.
    #[tokio::test]
    async fn csrf_routes_return_rotated_bearer_token_as_header() {
        let config = SessionConfig {
            sliding_policy: SlidingPolicy::Always,
            ..Default::default()
        };
        let sessions = Arc::new(SessionService::new(
            Arc::new(MemoryStorage::new()),
            Arc::new(SnowflakeService::new(0)),
            config,
        ));
        let secret = SecretString::from("test-secret");
        let created = sessions
            .create_session(
                Snowflake(1),
                &secret,
                UserDevice {
                    raw_user_agent: "test".into(),
                    _device_type: DeviceType::Desktop,
                    _os: None,
                    _browser: None,
                    ip: None,
                },
            )
            .await
            .unwrap();

        let state = SessionMiddlewareState::new(sessions, secret, false);
        let app = Router::new()
            .route("/", post(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(
                state,
                session_csrf_middleware,
            ))
            .layer(CookieManagerLayer::new());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let response = reqwest::Client::new()
            .post(format!("http://{addr}/"))
            .bearer_auth(&created.raw_token)
            .send()
            .await
            .unwrap();

        assert!(response.status().is_success());
        let new_token = response
            .headers()
            .get("X-New-Session-Token")
            .expect("rotated token should be returned to a Bearer client");
        assert_ne!(new_token.to_str().unwrap(), created.raw_token);
        assert!(response.headers().get(header::SET_COOKIE).is_none());
    }
}
