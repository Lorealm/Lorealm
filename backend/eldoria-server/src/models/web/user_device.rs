use std::str::FromStr;

use axum::http::HeaderMap;
use regex::Regex;

use crate::models::UserDeviceError;

pub struct UserDevice {
    pub ip: String,
    pub user_agent: String,
}

impl UserDevice {
    /// Extracts a user device from http headers
    pub fn from_http(headers: &HeaderMap) -> Self {
        let ip = headers
            .get("X-Forwarded-For")
            .or_else(|| headers.get("X-Real-IP"))
            .and_then(|value| value.to_str().ok())
            .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
            .unwrap_or_default();
        
        let user_agent = headers
            .get("User-Agent")
            .and_then(|value| value.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_default();

        Self { ip, user_agent }
    }

    /// Validates the structure of the IP address and user agent
    pub fn validate(&self) -> Result<(), UserDeviceError> {
        // Validate IP address (IPv4 or IPv6)
        let _ip = std::net::IpAddr::from_str(&self.ip)?;

        // Validate user agent (non-empty and contains typical characters)
        let ua_regex = Regex::new(r"^[A-Za-z0-9\s.,;:/()_-]+$")
            .map_err(UserDeviceError::UaRegexCompile)?;
        
        if !self.user_agent.is_empty() && !ua_regex.is_match(&self.user_agent) {
            return Err(UserDeviceError::InvalidUserAgent(self.user_agent.clone()));
        }

        if self.user_agent.is_empty() {
            return Err(UserDeviceError::EmptyUserAgent);
        }

        Ok(())
    }
}