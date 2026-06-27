use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    #[serde(default = "cleanup_interval")]
    pub cleanup_interval: u64,
}

fn cleanup_interval() -> u64 {
    300
}

impl Default for TaskConfig {
    fn default() -> Self {
        Self {
            cleanup_interval: cleanup_interval(),
        }
    }
}

impl Config for TaskConfig {
    fn key() -> &'static str {
        "tasks"
    }
}
