//! TODO: replace/integrate with [woothee](https://woothee.github.io/)?

use std::{net::IpAddr, sync::Arc};

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

use crate::{config::HostingConfig, core::AppState, extract::client_ip};

#[derive(Debug, Clone)]
pub struct UserDevice {
    pub raw_user_agent: String,
    pub device_type: DeviceType,
    pub os: Option<String>,
    pub browser: Option<String>,
    pub ip: Option<IpAddr>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceType {
    Desktop,
    Mobile,
    Tablet,
    Bot,
    Unknown,
}

impl FromRequestParts<Arc<AppState>> for UserDevice {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let ua = parts
            .headers
            .get("user-agent")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let lower = ua.to_lowercase();
        let device_type = classify_device(&lower);
        let os = extract_os(&lower);
        let browser = extract_browser(&lower);
        let trusted =
            client_ip::parse_trusted(&state.config().get::<HostingConfig>().trusted_proxies);
        let peer = parts
            .extensions
            .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            .map(|ci| ci.0.ip());
        let ip = client_ip::resolve(peer, &parts.headers, &trusted);

        Ok(UserDevice {
            raw_user_agent: ua,
            device_type,
            os,
            browser,
            ip,
        })
    }
}

fn classify_device(ua: &str) -> DeviceType {
    if ua.contains("bot") || ua.contains("crawler") || ua.contains("spider") {
        DeviceType::Bot
    } else if ua.contains("ipad") || ua.contains("tablet") {
        DeviceType::Tablet
    } else if ua.contains("mobile") || ua.contains("iphone") || ua.contains("android") {
        DeviceType::Mobile
    } else if ua.contains("windows") || ua.contains("macintosh") || ua.contains("linux") {
        DeviceType::Desktop
    } else {
        DeviceType::Unknown
    }
}

fn extract_os(ua: &str) -> Option<String> {
    if ua.contains("windows") {
        Some("Windows".into())
    } else if ua.contains("macintosh") || ua.contains("mac os") {
        Some("macOS".into())
    } else if ua.contains("iphone") || ua.contains("ipad") {
        Some("iOS".into())
    } else if ua.contains("android") {
        Some("Android".into())
    } else if ua.contains("linux") {
        Some("Linux".into())
    } else {
        None
    }
}

fn extract_browser(ua: &str) -> Option<String> {
    // Order matters — Chrome UA string contains "safari", Edge contains "chrome", etc.
    if ua.contains("edg/") || ua.contains("edge/") {
        Some("Edge".into())
    } else if ua.contains("opr/") || ua.contains("opera") {
        Some("Opera".into())
    } else if ua.contains("chrome") && !ua.contains("chromium") {
        Some("Chrome".into())
    } else if ua.contains("firefox") {
        Some("Firefox".into())
    } else if ua.contains("safari") {
        Some("Safari".into())
    } else {
        None
    }
}
