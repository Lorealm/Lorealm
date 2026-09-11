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
    /// Set `Secure` on the session/CSRF cookies. Requires HTTPS.
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default = "last_setup_version")]
    pub last_setup_version: Option<String>,
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

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            serve_frontend: false,
            frontend_dir: default_frontend_dir(),
            cookie_secure: false,
            last_setup_version: last_setup_version(),
        }
    }
}

impl Config for HostingConfig {
    fn key() -> &'static str {
        "hosting"
    }
}
