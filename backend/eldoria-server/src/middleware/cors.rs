use tower_http::cors::{Any, CorsLayer};

pub fn cors_layer(cfg: &crate::config::AppConfig) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(cfg.security.cors_origins.iter().map(|s| s.parse().unwrap()).collect::<Vec<_>>())
        .allow_methods(Any)
        .allow_headers(Any)
}