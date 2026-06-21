mod service;

use serde::Deserialize;
pub use service::TokenizerService;

#[derive(Deserialize)]
#[serde(tag = "type")]
enum CreateTokenizer {
    Import(Vec<u8>),
    Network(String)
}