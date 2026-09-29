use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Log entries kept per user per plugin; older ones are dropped.
    #[serde(default = "log_cap")]
    pub log_cap: u32,
    /// How long a plugin's `generate` may wait for the model in seconds.
    #[serde(default = "generate_timeout")]
    pub generate_timeout: u64,
}

fn log_cap() -> u32 {
    500
}

fn generate_timeout() -> u64 {
    120
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            log_cap: log_cap(),
            generate_timeout: generate_timeout(),
        }
    }
}

impl Config for PluginConfig {
    fn key() -> &'static str {
        "plugins"
    }
}
