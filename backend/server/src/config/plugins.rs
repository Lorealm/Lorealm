use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Log entries kept per user per plugin; older ones are dropped.
    #[serde(default = "log_cap")]
    pub log_cap: u32,
}

fn log_cap() -> u32 {
    500
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self { log_cap: log_cap() }
    }
}

impl Config for PluginConfig {
    fn key() -> &'static str {
        "plugins"
    }
}
