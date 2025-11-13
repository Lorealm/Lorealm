use std::sync::Arc;

use axum::{Router, extract::DefaultBodyLimit, routing::post};
use tower_cookies::CookieManagerLayer;

use crate::{
    app_state::AppState,
    middleware::authentication::{SessionMiddlewareState, session_csrf_middleware},
};

mod external;

pub(super) fn char_router(
    app_state: Arc<AppState>,
    middleware_state: SessionMiddlewareState,
) -> axum::Router {
    Router::new()
        .route("/import", post(external::import_card))
        .route_layer(DefaultBodyLimit::max(
            app_state.config().max_card_upload_size_mb * 1024 * 1024,
        ))
        .layer(axum::middleware::from_fn_with_state(
            middleware_state.clone(),
            session_csrf_middleware,
        ))
        .layer(CookieManagerLayer::new())
        .with_state(app_state)
}
