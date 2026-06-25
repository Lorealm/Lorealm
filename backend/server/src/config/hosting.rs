use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone)]
pub struct HostingConfig {
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "last_setup_version")]
    pub last_setup_version: Option<String>
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

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            last_setup_version: last_setup_version()
        }
    }
}

impl Config for HostingConfig {
    fn key() -> &'static str {
        "hosting"
    }
}
