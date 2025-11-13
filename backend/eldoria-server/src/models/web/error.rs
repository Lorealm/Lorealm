use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserDeviceError {
    #[error("Invalid IP address: {0}")]
    InvalidIp(#[from] std::net::AddrParseError),
    #[error("Failed to compile user agent regex: {0}")]
    UaRegexCompile(#[from] regex::Error),
    #[error("Invalid user agent format: {0}")]
    InvalidUserAgent(String),
    #[error("User agent is empty")]
    EmptyUserAgent,
}