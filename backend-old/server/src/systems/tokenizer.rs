use sentencepiece::SentencePieceProcessor;
use serde::Deserialize;

const CHARACTERS_PER_TOKEN_RATIO: f32 = 3.35;


#[derive(Debug)]
pub struct TokenizerError {
    pub message: String,
    /// The guesstimated token count if the encoding failed
    pub guesstimation: Option<usize>
}

impl From<Box<dyn std::error::Error>> for TokenizerError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        TokenizerError {
            message: value.to_string(),
            guesstimation: None,
        }
    }
}

impl From<serde_json::Error> for TokenizerError {
    fn from(value: serde_json::Error) -> Self {
        TokenizerError { message: value.to_string(), guesstimation: None }
    }
}

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

use crate::resources::resource_config::FileLike;
use crate::resources::resource_manager::ResourceManager;
use crate::state::AppState;

#[async_trait::async_trait]
pub trait Tokenizer: Send + Sync {
    async fn new(
        name: String,
        model_path: &str,
        fallback_model_path: Option<&str>,
    ) -> Self
    where
        Self: Sized;

    async fn count_tokens(&self, input: &str) -> Result<usize, TokenizerError>;

    async fn get_chunks(&self, ids: Vec<u32>) -> Result<Vec<String>, TokenizerError>;
}

#[async_trait::async_trait]
pub trait LazyTokenizer: Send + Sync + Tokenizer {
    async fn load(&mut self);
    async fn unload(&mut self);
}

pub struct SentencePieceTokenizer {
    name: String,
    main_path: PathBuf,
    fallback_path: PathBuf,
    tokenizer: Option<SentencePieceProcessor>
}

#[async_trait::async_trait]
impl Tokenizer for SentencePieceTokenizer {
    async fn new(name:String, model_path: &str, fallback_model_path: Option<&str>) -> Self {
        Self {
            name,
            main_path: PathBuf::from(model_path),
            fallback_path: PathBuf::from(fallback_model_path.unwrap_or("")),
            tokenizer: None
        }
    }

    async fn count_tokens(
        &self,
        input: &str,
    ) -> Result<usize, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            if let Ok(tokens) = tokenizer.encode(input) {
                Ok(tokens.len())
            } else {
                Err(TokenizerError {
                    message: "SENTENCEPIECE: Failed to encode input".to_string(),
                    guesstimation: Some(guess_token_count(input)),
                })
            }
        } else {
            let message = format!("Tokenizer '{}' not loaded", self.name);
            Err(TokenizerError { message: message, guesstimation: Some(guess_token_count(input)) })
        }
        
    }

    async fn get_chunks(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<String>, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            let mut chunks = vec![];
            for id in ids {
                if let Ok(chunk) = tokenizer.decode_piece_ids(&[id]) {
                    chunks.push(chunk);
                }
            }
            Ok(chunks)
        } else {
            let message = format!("Tokenizer '{}' not loaded", self.name);
            Err(TokenizerError { message: message, guesstimation: None })
        }
        
    }
}

#[async_trait::async_trait]
impl LazyTokenizer for SentencePieceTokenizer {
    async fn load(&mut self) {
        if self.tokenizer.is_some() {
            return;
        }

        let result = SentencePieceProcessor::open(&self.main_path);
        if let Ok(tokenizer) = result {
            self.tokenizer = Some(tokenizer);
            return;
        } else {
            let fallback_result = SentencePieceProcessor::open(&self.fallback_path);
            if let Ok(tokenizer) = fallback_result {
                self.tokenizer = Some(tokenizer);
                return;
            }
        }

        log::warn!("Tokenizer '{}' was not loaded correctly. Possibly due to an invalid path.", self.name)
    }

    async fn unload(&mut self) {
        if self.tokenizer.is_some() {
            self.tokenizer = None
        }
    }
}

pub struct HuggingFaceTokenizer {
    name: String,
    main_path: PathBuf,
    fallback_path: PathBuf,
    tokenizer: Option<tokenizers::Tokenizer>
}

#[async_trait::async_trait]
impl Tokenizer for HuggingFaceTokenizer {
    async fn new(name: String, model_path: &str, fallback_path: Option<&str>) -> Self {
        Self { 
            name,
            main_path: PathBuf::from(model_path),
            fallback_path: PathBuf::from(fallback_path.unwrap_or("")),
            tokenizer: None 
        }
        // let model = match tokenizers::Tokenizer::from_file(model_path) {
        //     Ok(tok) => tok,
        //     Err(_) => {
        //         log::warn!("TOKENIZERS: Failed to load model from path. Using fallback.");
        //         log::debug!("TOKENIZERS: Used model path: {}", model_path);
        //         tokenizers::Tokenizer::from_file(model_path).unwrap()
        //     },
        // };

        // HuggingFaceTokenizer {
        //     tokenizer: model
        // }
    }

    async fn count_tokens(
        &self,
        input: &str,
    ) -> Result<usize, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            if let Ok(tokens) = tokenizer.encode(input, false) {
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
        } else {
            let message = format!("Tokenizer '{}' not loaded", self.name);
            Err(TokenizerError { message: message, guesstimation: Some(guess_token_count(input)) })
        }
    }

    async fn get_chunks(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<String>, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            let mut chunks = vec![];
                for id in ids {
                    if let Ok(chunk) = tokenizer.decode(&[id], false) {
                        chunks.push(chunk);
                    }
                }
                
            Ok(chunks)
        } else {
            let message = format!("Tokenizer '{}' not loaded", self.name);
            Err(TokenizerError { message: message, guesstimation: None })
        }
    }
}

#[async_trait::async_trait]
impl LazyTokenizer for HuggingFaceTokenizer {
    async fn load(&mut self) {
        if self.tokenizer.is_some() {
            return;
        }

        let model = match tokenizers::Tokenizer::from_file(&self.main_path) {
            Ok(tok) => tok,
            Err(_) => {
                log::warn!("TOKENIZERS: Failed to load model from path. Using fallback.");
                log::debug!("TOKENIZERS: Used model path: {}", &self.main_path.to_str().unwrap());
                tokenizers::Tokenizer::from_file(&self.fallback_path).unwrap()
            },
        };

        self.tokenizer = Some(model)
    }

    async fn unload(&mut self) {
        if self.tokenizer.is_some() {
            self.tokenizer = None
        }
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

#[async_trait::async_trait]
impl Tokenizer for TiktokenTokenizer {
    /// This specific implementation uses a model name instead of a path. Fallback is not supported.
    async fn new(_: String, model_name: &str, _: Option<&str>) -> Self {
        Self {
            tokenizer: None,
            tokenizer_name: model_name.to_string()
        }
    }

    async fn count_tokens(
        &self,
        input: &str,
    ) -> Result<usize, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            let tokens = tokenizer.encode_with_special_tokens(input);
            Ok(tokens.len())
        }
        else {
            Err(TokenizerError {
                message: "TIKTOKEN: Failed to create tokenizer: Not loaded".to_string(),
                guesstimation: Some(guess_token_count(input))
            })
        }
        
    }

    async fn get_chunks(
        &self,
        ids: Vec<u32>,
    ) -> Result<Vec<String>, TokenizerError> {
        if let Some(tokenizer) = &self.tokenizer {
            let mut chunks = vec![];
            for id in ids {
                if let Ok(chunk) = tokenizer.decode([id].to_vec()) {
                    chunks.push(chunk);
                }
            }
            Ok(chunks)
        } else {
            Err(TokenizerError {
                message: "TIKTOKEN: Failed to create tokenizer: Not loaded".to_string(),
                guesstimation: None
            })
        }
    }
}

#[async_trait::async_trait]
impl LazyTokenizer for TiktokenTokenizer {
    async fn load(&mut self) {
        if self.tokenizer.is_some() {
            return;
        }

        if let Ok(model) = self.get_model() {
            self.tokenizer = Some(model);
            return;
        }

        log::warn!("Tokenizer '{}' was not loaded correctly. Possibly due to an invalid path.", self.tokenizer_name)
    }

    async fn unload(&mut self) {
        if self.tokenizer.is_some() {
            self.tokenizer = None
        }
    }
}

#[derive(Deserialize)]
struct TokenizerPaths {
    main: String,
    fallback: Option<String>,
}

#[derive(Deserialize)]
struct TokenizerDefinition {
    tokenizer: String,
    patterns: Vec<String>,
    format: String,
    paths: Option<TokenizerPaths> // Not needed for tiktoken
}

#[derive(Deserialize)]
struct TokenizerConfig {
    _exact_self_matches: Vec<String>,
    mappings: Vec<TokenizerDefinition>
}

pub struct TokenizerManager {
    tokenizers: HashMap<String, Box<dyn LazyTokenizer>>,
    loaded_tokenizers: Vec<String>,
    synonyms: HashMap<String, String>
}

impl TokenizerManager {
    pub fn new() -> Self {
        Self {
            tokenizers: HashMap::new(),
            loaded_tokenizers: Vec::new(),
            synonyms: HashMap::new()
        }
    }

    pub async fn count_tokens(
        &mut self,
        tokenizer_name: &String,
        content: &String,
    ) -> Result<usize, TokenizerError> {
        let tokenizer = self.tokenizer_from_name(tokenizer_name).await?;

        tokenizer.count_tokens(content).await
    }

    async fn tokenizer_from_name(
        &mut self,
        name: &str,
    ) -> Result<&mut Box<dyn LazyTokenizer>, TokenizerError> {
        if let Some(tokenizer_name) = self.synonyms.get(name) {
            // Clone so we can drop the immutable borrow
            let tokenizer_name = tokenizer_name.clone();

            if let Some(tokenizer) = self.tokenizers.get_mut(&tokenizer_name) {
                if self.loaded_tokenizers.contains(&tokenizer_name) {
                    Ok(tokenizer)
                } else {
                    tokenizer.as_mut().load().await;
                    self.loaded_tokenizers.push(tokenizer_name);
                    Ok(tokenizer)
                }
            } else {
                Err(TokenizerError {
                    message: format!("No instantiated tokenizer with name '{}' was found", name),
                    guesstimation: None,
                })
            }
        } else {
            Err(TokenizerError {
                message: format!("Tokenizer with name '{}' was not found in local cache", name),
                guesstimation: None,
            })
        }
    }


    pub async fn load_definition_file(&mut self, file: Box<dyn FileLike>, app_state: &AppState) -> Result<(), TokenizerError> {
        let config: TokenizerConfig = serde_json::from_reader(file)?;
        let mut map = HashMap::new();
        let mut synonyms = HashMap::new();
        
        let resource_manager = app_state.resource_manager.lock().await;

        for mapping in config.mappings {
            let name = &mapping.tokenizer.clone();
            let tokenizer = match mapping.format.as_str() {
                "tiktoken" => TokenizerManager::create_tiktoken_tokenizer(&mapping).await,
                "diffusers" => TokenizerManager::create_diffusers_tokenizer(&mapping, &resource_manager).await,
                "sentencepiece" => TokenizerManager::create_sentencepiece_tokenizer(&mapping, &resource_manager).await,
                &_ => panic!()
            };

            if let Err(e) = tokenizer {
                log::error!("TOKENIZERS: Failed to create tokenizer {}: {}", name, e);
                continue;
            }
            
            for synonym in mapping.patterns {
                log::trace!("Found synonym {} for {}", &synonym, &name);
                synonyms.insert(synonym, name.clone());
            }

            log::info!("TOKENIZERS: Created tokenizer: {}", name);
            map.insert(name.clone(), tokenizer.unwrap());
        }

        drop(resource_manager);

        self.tokenizers = map;
        self.loaded_tokenizers = Vec::new();
        self.synonyms = synonyms;
        Ok(())
    }

    async fn create_tiktoken_tokenizer(def: &TokenizerDefinition) -> Result<Box<dyn LazyTokenizer>, Box<dyn std::error::Error>> {
        Ok(Box::new(TiktokenTokenizer::new(def.tokenizer.clone(), &def.tokenizer, None).await))
    }

    async fn create_diffusers_tokenizer(def: &TokenizerDefinition, resource_manager: &ResourceManager) -> Result<Box<dyn LazyTokenizer>, Box<dyn std::error::Error>> {
        let paths = def.paths.as_ref().unwrap();

        let (main, fallback) = get_paths(resource_manager, paths)?;

        Ok(Box::new(HuggingFaceTokenizer::new(def.tokenizer.clone(), &main, fallback.as_deref()).await))
    }

    async fn create_sentencepiece_tokenizer(def: &TokenizerDefinition, resource_manager: &ResourceManager) -> Result<Box<dyn LazyTokenizer>, Box<dyn std::error::Error>> {
        let paths = def.paths.as_ref().unwrap();

        let (main, _) = get_paths(resource_manager, paths)?;

        Ok(Box::new(SentencePieceTokenizer::new(def.tokenizer.clone(), &main, None).await))
    }
}

fn get_paths(resource_manager: &ResourceManager, paths: &TokenizerPaths) -> Result<(String, Option<String>), Box<dyn std::error::Error>> {
    let path_count = if paths.fallback.is_some() { 2 } else { 1 };

    let mut request = resource_manager.request_builder()
        .with_credentials(Some(env::var("SYSTEM_FILE_ACCESS_TOKEN").unwrap_or(String::new())), true)
        .with_file(paths.main.clone());

    if path_count == 2 {
        request = request.with_file(paths.fallback.as_ref().unwrap());
    }

    let files = request.request_qualified_paths()?;
    let paths = match &files.len() {
        1 => (files[0].clone().to_string_lossy().to_string(), None),
        2 => (files[0].clone().to_string_lossy().to_string(), Some(files[1].clone().to_string_lossy().to_string())),
        _ => return Err(format!("Got an unexpected number of files when creating tokenizer: {}", &files.len()).into())
    };

    Ok(paths)
}

/// Rougly estimates the token count of a given string based on a static ratio
pub fn guess_token_count(input: &str) -> usize {
    let count = input.chars().count() as f32;
    let tokens = count / CHARACTERS_PER_TOKEN_RATIO;
    tokens.ceil() as usize
}