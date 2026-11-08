use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone)]
pub struct HostingConfig {
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default)]
    pub serve_frontend: bool,
    #[serde(default = "default_frontend_dir")]
    pub frontend_dir: PathBuf,
    pub worker_id: i64,
    /// Set `Secure` on the session/CSRF cookies. Requires HTTPS.
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default = "last_setup_version")]
    pub last_setup_version: Option<String>,
    /// Reverse proxies whose `X-Forwarded-For` / `X-Real-IP` headers are
    /// believed, as addresses or CIDR ranges (e.g. `172.16.0.0/12` for Docker).
    /// Requests from anywhere else are identified by their connecting address.
    #[serde(default = "default_trusted_proxies")]
    pub trusted_proxies: Vec<String>,
}

fn default_ip() -> String {
    "127.0.0.1".to_owned()
}
fn default_port() -> u16 {
    1551
}
fn last_setup_version() -> Option<String> {
    None
}
fn default_frontend_dir() -> PathBuf {
    PathBuf::from("../frontend/apps/web/dist")
}
fn default_trusted_proxies() -> Vec<String> {
    vec!["127.0.0.1".to_owned(), "::1".to_owned()]
}
fn default_worker_id() -> i64 {
    0
}

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            serve_frontend: false,
            frontend_dir: default_frontend_dir(),
            worker_id: default_worker_id(),
            cookie_secure: false,
            last_setup_version: last_setup_version(),
            trusted_proxies: default_trusted_proxies(),
        }
    }
}

impl Config for HostingConfig {
    fn key() -> &'static str {
        "hosting"
    }
}
