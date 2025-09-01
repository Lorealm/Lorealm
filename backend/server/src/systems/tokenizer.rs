use sentencepiece::SentencePieceProcessor;

const CHARACTERS_PER_TOKEN_RATIO: f32 = 3.35;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenizerType {
    None = 0,
    GPT2 = 1,
    OpenAI = 2,
    Llama = 3,
    Nerd = 4,
    Nerd2 = 5,
    Mistral = 6,
    Yi = 7,
    ApiTextGenerationWebUI = 8,
    ApiKobold = 9,
    Claude = 10,
    Llama3 = 11,
    Gemma = 12,
    Jamba = 13,
    Qwen2 = 14,
    CommandR = 15,
    Nemo = 16,
    Deepseek = 17,
    CommandA = 18,
    BestMatch = isize::MAX
}

pub const ENCODE_TOKENIZERS: [TokenizerType; 11] = [
    TokenizerType::Llama,
    TokenizerType::Mistral,
    TokenizerType::Yi,
    TokenizerType::Llama3,
    TokenizerType::Gemma,
    TokenizerType::Jamba,
    TokenizerType::Qwen2,
    TokenizerType::CommandR,
    TokenizerType::Nemo,
    TokenizerType::Deepseek,
    TokenizerType::CommandA,
];

pub struct TokenizerError {
    pub message: String,
    /// The guesstimated token count if the encoding failed
    pub guesstimation: Option<usize>
}

use std::future::Future;
use std::pin::Pin;

pub trait Tokenizer: Send + Sync {
    fn new(model_path: &str, fallback_model_path: Option<&str>) -> impl Future<Output = Self> + Send
    where
        Self: Sized;

    fn count_tokens<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<usize, TokenizerError>> + Send + 'a>>;

    fn get_chunks<'a>(
        &'a self,
        ids: Vec<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, TokenizerError>> + Send + 'a>>;
}

pub struct SentencePieceTokenizer {
    tokenizer: SentencePieceProcessor
}

// Example for SentencePieceTokenizer (apply similarly to others)
impl Tokenizer for SentencePieceTokenizer {
    fn new(model_path: &str, fallback_model_path: Option<&str>) -> impl Future<Output = Self> + Send {
        async { todo!() }
    }

    fn count_tokens<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<usize, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            if let Ok(tokens) = self.tokenizer.encode(input) {
                Ok(tokens.len())
            } else {
                Err(TokenizerError {
                    message: "SENTENCEPIECE: Failed to encode input".to_string(),
                    guesstimation: Some(guess_token_count(input)),
                })
            }
        })
    }

    fn get_chunks<'a>(
        &'a self,
        ids: Vec<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            let mut chunks = vec![];
            for id in ids {
                if let Ok(chunk) = self.tokenizer.decode_piece_ids(&[id]) {
                    chunks.push(chunk);
                }
            }
            Ok(chunks)
        })
    }
}

pub struct HuggingFaceTokenizer {
    tokenizer: tokenizers::Tokenizer
}

impl Tokenizer for HuggingFaceTokenizer {
    async fn new(model_path: &str, fallback_model_path: Option<&str>) -> Self {
        todo!()
    }

    fn count_tokens<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<usize, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            if let Ok(tokens) = self.tokenizer.encode(input, false) {
            Ok(tokens.len())
            }
            else {
                Err(
                    TokenizerError {
                        message: "TOKENIZERS: Failed to encode input".to_string(),
                        guesstimation: Some(guess_token_count(input))
                    }
                )
            }
        })
    }

    fn get_chunks<'a>(
        &'a self,
        ids: Vec<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            let mut chunks = vec![];
            for id in ids {
                if let Ok(chunk) = self.tokenizer.decode(&[id], false) {
                    chunks.push(chunk);
                }
            }
            
            Ok(chunks)
        })
    }
}

pub struct TiktokenTokenizer {
    tokenizer: Option<tiktoken_rs::CoreBPE>,
    tokenizer_name: String
}

impl TiktokenTokenizer {
    fn get_model(&self) -> Result<tiktoken_rs::CoreBPE, anyhow::Error> {
        if self.tokenizer.is_none() {
            tiktoken_rs::get_bpe_from_model(&self.tokenizer_name)
        } else {
            Ok(self.tokenizer.as_ref().unwrap().clone())
        }
    }
}

impl Tokenizer for TiktokenTokenizer {
    /// This specific implementation uses a model name instead of a path. Fallback is not supported.
    async fn new(model_name: &str, _: Option<&str>) -> Self {
        Self {
            tokenizer: None,
            tokenizer_name: model_name.to_string()
        }
    }

    fn count_tokens<'a>(
        &'a self,
        input: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<usize, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            let tokenizer = self.get_model().map_err(|e| TokenizerError {
                message: format!("TIKTOKEN: Failed to create tokenizer: {}", e),
                guesstimation: Some(guess_token_count(input))
            })?;
            
            let tokens = tokenizer.encode_with_special_tokens(input);
            Ok(tokens.len())
        })
    }

    fn get_chunks<'a>(
        &'a self,
        ids: Vec<u32>,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<String>, TokenizerError>> + Send + 'a>> {
        Box::pin(async move {
            let mut chunks = vec![];
            let tokenizer = self.get_model().map_err(|e| TokenizerError {
                message: format!("TIKTOKEN: Failed to create tokenizer: {}", e),
                guesstimation: None
            })?;
            
            for id in ids {
                if let Ok(chunk) = tokenizer.decode([id].to_vec()) {
                    chunks.push(chunk);
                }
            }
            
            Ok(chunks)
        })
    }
}

/// Rougly estimates the token count of a given string based on a static ratio
pub fn guess_token_count(input: &str) -> usize {
    let count = input.chars().count() as f32;
    let tokens = count / CHARACTERS_PER_TOKEN_RATIO;
    tokens.ceil() as usize
}