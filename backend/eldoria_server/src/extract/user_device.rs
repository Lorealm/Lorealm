use std::net::IpAddr;

use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};

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

impl<S> FromRequestParts<S> for UserDevice
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
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
        let ip = extract_ip(parts);

        Ok(UserDevice {
            raw_user_agent: ua,
            device_type,
            os,
            browser,
            ip,
        })
    }
}

fn extract_ip(parts: &Parts) -> Option<IpAddr> {
    // 1. Check proxy headers (order by trust preference)
    let proxy_headers = ["x-forwarded-for", "x-real-ip", "cf-connecting-ip"];

    for header in proxy_headers {
        if let Some(value) = parts.headers.get(header).and_then(|v| v.to_str().ok()) {
            // X-Forwarded-For can be a comma-separated list; take the first (client) IP
            let candidate = value.split(',').next().unwrap_or(value).trim();
            if let Ok(ip) = candidate.parse::<IpAddr>() {
                return Some(ip);
            }
        }
    }

    // 2. Fall back to the connected peer address
    parts
        .extensions
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
        .map(|ci| ci.0.ip())
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
