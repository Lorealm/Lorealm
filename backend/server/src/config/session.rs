use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Defines how session expiration should be extended on activity
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlidingPolicy {
    /// Never extend expiration - session has fixed lifetime from creation
    #[default]
    Fixed,

    /// Always extend expiration on every activity
    Always,

    /// Only extend if session is within the specified window before expiry
    Window(i64),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CsrfRotationPolicy {
    /// Never rotate - token lasts for session lifetime
    /// Best for: SPAs, most applications
    #[default]
    Never,

    /// Rotate every N seconds
    /// Best for: Balanced security/usability
    Periodic(i64),

    /// Rotate after N state-changing requests
    /// Best for: Rate-limited rotation
    AfterRequests(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[serde(default = "token_length")]
    pub token_length: usize,
    #[serde(default = "csrf_token_length")]
    pub csrf_token_length: usize,
    #[serde(default = "ttl_seconds")]
    pub ttl_seconds: i64,
    #[serde(default = "sliding_policy")]
    pub sliding_policy: SlidingPolicy,
    #[serde(default = "csrf_rotation_policy")]
    pub csrf_rotation_policy: CsrfRotationPolicy,
}

fn token_length() -> usize {
    32
}

fn csrf_token_length() -> usize {
    32
}

fn ttl_seconds() -> i64 {
    60 * 60 * 24 // 24 hours
}

fn sliding_policy() -> SlidingPolicy {
    SlidingPolicy::Window(60 * 60)
}

fn csrf_rotation_policy() -> CsrfRotationPolicy {
    CsrfRotationPolicy::Never
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            token_length: token_length(),
            csrf_token_length: csrf_token_length(),
            ttl_seconds: ttl_seconds(),
            sliding_policy: sliding_policy(),
            csrf_rotation_policy: csrf_rotation_policy(),
        }
    }
}

impl Config for SessionConfig {
    fn key() -> &'static str {
        "sessions"
    }
}
