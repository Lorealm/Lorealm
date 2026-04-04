use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default = "password_length")]
    pub password_length: (usize, usize),
    #[serde(default = "username_length")]
    pub username_length: (usize, usize),
    #[serde(default = "disable_register")]
    pub disable_register: bool,
}

fn password_length() -> (usize, usize) {
    (8, 64)
}

fn username_length() -> (usize, usize) {
    (3, 32)
}

fn disable_register() -> bool {
    false
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            password_length: password_length(),
            username_length: username_length(),
            disable_register: disable_register(),
        }
    }
}

impl Config for UserConfig {
    fn key() -> &'static str {
        "users"
    }
}
