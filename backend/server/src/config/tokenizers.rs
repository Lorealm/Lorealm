use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizerConfig {
    #[serde(default = "max_tokenizer_size_mb")]
    pub max_tokenizer_size_mb: u64,
    #[serde(default = "tokenizer_ttl")]
    pub tokenizer_ttl: i64,
    #[serde(default = "default_tokenizers")]
    pub default_tokenizers: Vec<String>,
}

fn max_tokenizer_size_mb() -> u64 {
    35
}

fn tokenizer_ttl() -> i64 {
    300
}

fn default_tokenizers() -> Vec<String> {
    vec![
        "hf:google/gemma-4-12B",
        "hf:deepseek-ai/DeepSeek-V4-Pro",
        "hf:zai-org/GLM-5.2",
    ]
    .iter()
    .map(|t| t.to_string())
    .collect()
}

impl Default for TokenizerConfig {
    fn default() -> Self {
        Self {
            max_tokenizer_size_mb: max_tokenizer_size_mb(),
            default_tokenizers: default_tokenizers(),
            tokenizer_ttl: tokenizer_ttl(),
        }
    }
}

impl Config for TokenizerConfig {
    fn key() -> &'static str {
        "tokenizers"
    }
}
