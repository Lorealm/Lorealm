use thiserror::Error;

#[derive(Error, Debug)]
pub enum CardParseError {
    #[error("not a PNG file")]
    InvalidPng,

    #[error("malformed PNG chunk structure")]
    InvalidChunkFormat,

    #[error("no tEXt chunk found with identifier '{0}'")]
    ChunkNotFound(String),

    #[error("base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("base64 encode error: {0}")]
    Base64Encode(#[from] base64::EncodeSliceError),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
