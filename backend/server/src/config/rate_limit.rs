use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Limit {
    pub burst: u32,
    pub period_secs: u64,
}

impl Limit {
    pub const fn new(burst: u32, period_secs: u64) -> Self {
        Self { burst, period_secs }
    }
}

/// Limits on the routes anyone can reach without signing in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "enabled")]
    pub enabled: bool,
    /// Every login attempt from one address.
    #[serde(default = "login_per_ip")]
    pub login_per_ip: Limit,
    /// Failed logins for one username or email, from any address.
    #[serde(default = "login_failures_per_account")]
    pub login_failures_per_account: Limit,
    /// Every registration attempt from one address.
    #[serde(default = "register_per_ip")]
    pub register_per_ip: Limit,
}

fn enabled() -> bool {
    true
}
fn login_per_ip() -> Limit {
    Limit::new(20, 300)
}
fn login_failures_per_account() -> Limit {
    Limit::new(10, 900)
}
fn register_per_ip() -> Limit {
    Limit::new(5, 3600)
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: enabled(),
            login_per_ip: login_per_ip(),
            login_failures_per_account: login_failures_per_account(),
            register_per_ip: register_per_ip(),
        }
    }
}

impl Config for RateLimitConfig {
    fn key() -> &'static str {
        "rate_limits"
    }
}
