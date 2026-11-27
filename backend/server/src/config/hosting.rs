use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Serialize, Deserialize, Clone)]
pub struct HostingConfig {
    #[serde(default = "default_ip")]
    pub ip: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_serve_frontend")]
    pub serve_frontend: bool,
    #[serde(default = "default_frontend_dir")]
    pub frontend_dir: PathBuf,
    pub worker_id: i64,
    /// Set `Secure` on the session/CSRF cookies. Requires HTTPS.
    #[serde(default)]
    pub cookie_secure: bool,
    #[serde(default = "last_setup_version")]
    pub last_setup_version: Option<String>,
    /// Reverse proxies whose `X-Forwarded-For` / `X-Real-IP` headers are
    /// believed, as addresses or CIDR ranges (e.g. `172.16.0.0/12` for Docker).
    /// Requests from anywhere else are identified by their connecting address.
    #[serde(default = "default_trusted_proxies")]
    pub trusted_proxies: Vec<String>,
    /// Open files the server may hold at once.
    #[serde(default)]
    pub max_open_files: OpenFileLimit,
    /// How much memory the message cache can use in MB. 0 = disabled.
    #[serde(default = "default_message_cache_mb")]
    pub message_cache_mb: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OpenFileLimit {
    /// Raise the soft limit to the hard limit.
    #[default]
    Max,
    /// Keep whatever limit the process started with.
    System,
    /// Raise the soft limit to this, capped by the hard limit.
    Count(u64),
}

impl Serialize for OpenFileLimit {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Max => s.serialize_str("max"),
            Self::System => s.serialize_str("system"),
            Self::Count(n) => s.serialize_u64(*n),
        }
    }
}

impl<'de> Deserialize<'de> for OpenFileLimit {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Count(u64),
            Word(String),
        }
        match Raw::deserialize(d)? {
            Raw::Count(n) => Ok(Self::Count(n)),
            Raw::Word(w) if w.eq_ignore_ascii_case("max") => Ok(Self::Max),
            Raw::Word(w) if w.eq_ignore_ascii_case("system") => Ok(Self::System),
            Raw::Word(w) => Err(serde::de::Error::custom(format!(
                "max_open_files must be `max`, `system` or a number, not `{w}`"
            ))),
        }
    }
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
fn default_serve_frontend() -> bool {
    true
}
fn default_frontend_dir() -> PathBuf {
    PathBuf::from("web")
}
fn default_trusted_proxies() -> Vec<String> {
    vec!["127.0.0.1".to_owned(), "::1".to_owned()]
}
fn default_worker_id() -> i64 {
    0
}
fn default_message_cache_mb() -> usize {
    64
}

impl Default for HostingConfig {
    fn default() -> Self {
        Self {
            ip: default_ip(),
            port: default_port(),
            serve_frontend: default_serve_frontend(),
            frontend_dir: default_frontend_dir(),
            worker_id: default_worker_id(),
            cookie_secure: false,
            last_setup_version: last_setup_version(),
            trusted_proxies: default_trusted_proxies(),
            max_open_files: OpenFileLimit::default(),
            message_cache_mb: default_message_cache_mb(),
        }
    }
}

impl Config for HostingConfig {
    fn key() -> &'static str {
        "hosting"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_file_limit_reads_words_and_numbers() {
        let read = |v: serde_json::Value| serde_json::from_value::<OpenFileLimit>(v);
        assert_eq!(read("max".into()).unwrap(), OpenFileLimit::Max);
        assert_eq!(read("System".into()).unwrap(), OpenFileLimit::System);
        assert_eq!(read(65536.into()).unwrap(), OpenFileLimit::Count(65536));
        assert!(read("lots".into()).is_err());
    }

    #[test]
    fn open_file_limit_round_trips() {
        for limit in [
            OpenFileLimit::Max,
            OpenFileLimit::System,
            OpenFileLimit::Count(4096),
        ] {
            let value = serde_json::to_value(limit).unwrap();
            assert_eq!(
                serde_json::from_value::<OpenFileLimit>(value).unwrap(),
                limit
            );
        }
    }

    #[test]
    fn older_configs_default_to_max() {
        let config: HostingConfig =
            serde_json::from_value(serde_json::json!({ "worker_id": 0 })).unwrap();
        assert_eq!(config.max_open_files, OpenFileLimit::Max);
    }
}
