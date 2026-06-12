use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Http(reqwest::Error),
    Json(serde_json::Error),
    ApiError { status: u16, message: String },
    InvalidParameter(String),
    StreamError(String),
    Provider(String),
    Tool(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(e) => write!(f, "HTTP error: {}", e),
            Error::Json(e) => write!(f, "JSON error: {}", e),
            Error::ApiError { status, message } => {
                write!(f, "API error ({}): {}", status, message)
            }
            Error::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            Error::StreamError(msg) => write!(f, "Stream error: {}", msg),
            Error::Provider(msg) => write!(f, "Provider error: {}", msg),
            Error::Tool(msg) => write!(f, "Tool error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Http(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err)
    }
}
