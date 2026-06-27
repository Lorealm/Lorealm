use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterConfig {
    #[serde(default = "upload_limit_mb")]
    pub upload_limit_mb: usize,
    #[serde(default = "edit_session_ttl")]
    pub edit_session_ttl: u64,
}

fn upload_limit_mb() -> usize {
    30
}

fn edit_session_ttl() -> u64 {
    600
}

impl Default for CharacterConfig {
    fn default() -> Self {
        Self {
            upload_limit_mb: upload_limit_mb(),
            edit_session_ttl: edit_session_ttl(),
        }
    }
}

impl Config for CharacterConfig {
    fn key() -> &'static str {
        "characters"
    }
}
