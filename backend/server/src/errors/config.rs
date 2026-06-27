#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("unknown key")]
    UnknownKey,
    #[error("invalid: {0}")]
    Invalid(serde_json::Error),
    #[error("io error: {0}")]
    Io(std::io::Error),
}

impl ConfigError {
    pub fn client_message(&self) -> String {
        match self {
            ConfigError::UnknownKey => "unknown key".to_string(),
            ConfigError::Invalid(error) => format!("invalid: {error}"),
            ConfigError::Io(_) => format!("internal error"),
        }
    }
}
