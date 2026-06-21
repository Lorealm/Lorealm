use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone)]
pub struct HostingConfig {
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_ip() -> String {
    "127.0.0.1".to_owned()
}
fn default_port() -> u16 {
    1551
}

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
        }
    }
}

impl Config for HostingConfig {
    fn key() -> &'static str {
        "hosting"
    }
}
