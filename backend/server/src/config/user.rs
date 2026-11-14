use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinMax {
    pub min: usize,
    pub max: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    #[serde(default = "password_length")]
    pub password_length: MinMax,
    #[serde(default = "username_length")]
    pub username_length: MinMax,
    #[serde(default = "disable_register")]
    pub disable_register: bool,
    #[serde(default = "create_default_resources")]
    pub create_default_resources: bool,
}

fn password_length() -> MinMax {
    MinMax { min: 8, max: 64 }
}

fn username_length() -> MinMax {
    MinMax { min: 3, max: 32 }
}

fn disable_register() -> bool {
    true
}

fn create_default_resources() -> bool {
    true
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            password_length: password_length(),
            username_length: username_length(),
            disable_register: disable_register(),
            create_default_resources: create_default_resources(),
        }
    }
}

impl Config for UserConfig {
    fn key() -> &'static str {
        "users"
    }
}
