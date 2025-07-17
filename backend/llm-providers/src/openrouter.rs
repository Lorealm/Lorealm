use std::sync::{Arc, Mutex};

use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::samplers::{config::SamplerConfig, json::{Payload, ProviderPayload}, max_tokens::MaxTokensSampler, temperature::TemperatureSampler, top_p::TopPSampler, types::{Sampler, SamplerCollection}};

use super::{llm_provider::{LLMProvider, LLMProviderResponse}, types::Message};

static ENDPOINT: &str = "https://openrouter.ai/api/v1";
static PROVIDER_NAME: &str = "openrouter";
static MODEL_NAME: &str = "tngtech/deepseek-r1t2-chimera:free";
static API_KEY: &str = dotenv!("OPENROUTER_API_KEY");
pub struct OpenRouterProvider;

impl OpenRouterProvider {
    pub fn new() -> Self {
        Self
    }
}

impl LLMProvider for OpenRouterProvider {
    async fn generate(&self, prompt: Vec<Message>) -> Result<Box<dyn LLMProviderResponse>, reqwest::Error> {
        log::info!("Generating with OpenRouter");
        let mut options = OpenRouterSettings::new(MODEL_NAME.to_string(), prompt);
        let config = SamplerConfig {
            temperature: Some((1.0, false, 0.0)),
            max_tokens: Some(200),
            ..Default::default()
        };
        let samplers = OpenRouterSamplers::samplers(&config);

        let payload = options.with_samplers(&samplers);

        let client = reqwest::Client::new();
        let res = client.post(format!("{}/chat/completions", ENDPOINT))
            .json(&payload)
            .bearer_auth(API_KEY.to_string())
            .send()
            .await;

        let res = res.unwrap();
        let json = res.json::<OpenRouterResponse>().await;
        match json {
            Ok(response) => {
                println!("Response: {}", response.text());
                Ok(Box::new(response))
            },
            Err(e) => {
                println!("Error: {}", e);
                Err(e)
            },
        }
    }

    fn cancel(&self) {
        todo!()
    }
    
    async fn generate_stream(self: Arc<Self>, prompt: Vec<Message>) -> Result<impl Stream<Item = Result<Box<dyn LLMProviderResponse>, Box<dyn std::error::Error + Send + Sync>>> + Send + Unpin, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Streaming with OpenRouter");
        let mut options = OpenRouterSettings::new(MODEL_NAME.to_string(), prompt);
        
        // Set stream to true in the payload
        options.stream = Some(true);
        
        let config = SamplerConfig {
            temperature: Some((1.0, false, 0.0)),
            max_tokens: Some(200),
            ..Default::default()
        };
        let samplers = OpenRouterSamplers::samplers(&config);
        let payload = options.with_samplers(&samplers);
    
        let client = reqwest::Client::new();
        let res = client.post(format!("{}/chat/completions", ENDPOINT))
            .json(&payload)
            .bearer_auth(API_KEY.to_string())
            .send()
            .await?;
            
        // Create a buffer to accumulate partial SSE messages
        let buffer = Arc::new(Mutex::new(String::new()));
        
        // Track the current message being built (in case we receive partial JSON)
        let current_message = Arc::new(Mutex::new(String::new()));
        
        // Convert the response stream to our response type
        let stream = res.bytes_stream().map(move |bytes_result| {
            match bytes_result {
                Ok(bytes) => {
                    // Parse the SSE data from bytes
                    if let Ok(chunk_text) = std::str::from_utf8(&bytes) {
                        let mut buffer = buffer.lock().unwrap();
                        buffer.push_str(chunk_text);
                        
                        // Process complete lines in the buffer
                        let mut processed_responses = Vec::new();
                        
                        // Split by lines and process each line
                        while let Some(line_end) = buffer.find('\n') {
                            let line = buffer[..line_end].trim().to_string();
                            *buffer = buffer[line_end + 1..].to_string();
                            
                            // Handle SSE format which starts with "data: "
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                
                                if data == "[DONE]" {
                                    processed_responses.push(Ok(Box::new(OpenRouterStreamEnd {}) as Box<dyn LLMProviderResponse>));
                                    continue;
                                }
                                
                                // Add to current message if it's a continuation 
                                let mut current_message = current_message.lock().unwrap();
                                current_message.push_str(data);
                                
                                // Try to parse the accumulated message
                                match serde_json::from_str::<OpenRouterStreamChunk>(&current_message) {
                                    Ok(chunk) => {
                                        // Successfully parsed, clear the message buffer
                                        current_message.clear();
                                        
                                        // Only add non-empty chunks
                                        if !chunk.is_empty() {
                                            processed_responses.push(Ok(Box::new(chunk) as Box<dyn LLMProviderResponse>));
                                        }
                                    },
                                    Err(_) => {
                                        // Not a complete JSON message yet, continue accumulating
                                        // No error logged here as this is expected for partial messages
                                    }
                                }
                            }
                        }
                        
                        if !processed_responses.is_empty() {
                            // Return the first response and queue the rest (if any) for future processing
                            return processed_responses.remove(0);
                        }
                    }
                    
                    // Return an empty response that will be filtered out later
                    Ok(Box::new(OpenRouterStreamEmpty {}) as Box<dyn LLMProviderResponse>)
                },
                Err(e) => Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            }
        });
        
        // Use filter_map with a pinned boxed future to ensure it's Unpin
        let filtered_stream = stream.filter_map(|result| {
            Box::pin(async move {
                match result {
                    Ok(response) => {
                        // Check if it's our empty placeholder and filter it out if so
                        if response.is_empty() {
                            None
                        } else {
                            Some(Ok(response))
                        }
                    },
                    Err(e) => Some(Err(e)),
                }
            })
        });
        
        // Box the stream to make it Unpin
        Ok(Box::pin(filtered_stream))
    }
}

#[derive(Serialize, Deserialize)]
pub struct OpenRouterResponse {
    id: String,
    provider: String,
    model: String,
    object: String,
    created: i32,
    choices: Vec<OpenRouterChoices>,
    usage: OpenRouterUsage
}

#[derive(Serialize, Deserialize)]
pub struct OpenRouterChoices {
    logprobs: Option<String>,
    finish_reason: String,
    native_finish_reason: String,
    index: Option<i32>,
    message: OpenRouterMessage,
    reasoning: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct OpenRouterMessage {
    role: String,
    content: String,
    reasoning: Option<String>,
    refusal: Option<String>
}

#[derive(Serialize, Deserialize)]
pub struct OpenRouterUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32
}

impl LLMProviderResponse for OpenRouterResponse {
    fn text(&self) -> String {
        self.choices[0].message.content.clone()
    }
    
    fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }
    
    fn is_empty(&self) -> bool {
        todo!()
    }
}

pub struct OpenRouterSamplers;

impl SamplerCollection for OpenRouterSamplers {
    fn samplers(config: &SamplerConfig) -> Vec<Box<dyn Sampler>> {
        let mut samplers: Vec<Box<dyn Sampler>> = Vec::new();

        // Configure TemperatureSampler if enabled.
        if let Some((value, shift_enabled, shift_amount)) = config.temperature {
            let mut ts = TemperatureSampler::new(PROVIDER_NAME, value);
            ts.set_shift_policy(shift_enabled, shift_amount);
            samplers.push(Box::new(ts));
        }

        // Configure TopPSampler if enabled.
        if let Some(top_p_value) = config.top_p {
            // Assuming a similar new() exists.
            samplers.push(Box::new(TopPSampler::new(PROVIDER_NAME, top_p_value)));
        }

        // Configure MaxTokensSampler if enabled.
        if let Some(max_tokens_value) = config.max_tokens {
            // Assuming a similar new() exists.
            samplers.push(Box::new(MaxTokensSampler::new(PROVIDER_NAME, max_tokens_value)));
        }

        // Return only the samplers that were enabled by the configuration.
        samplers
    }
}


#[derive(Default)]
pub struct OpenRouterSettings {
    pub model: String,
    pub prompt: Vec<Message>,
    pub stream: Option<bool>,
    pub seed: Option<i32>
}

impl OpenRouterSettings {
    pub fn new(model: String, prompt: Vec<Message>) -> Self {
        Self {
            model,
            prompt,
            ..Default::default()
        }
    }

    
    pub fn stream(&mut self, stream: bool) -> &mut Self {
        self.stream = Some(stream);

        self
    }

    pub fn seed(&mut self, seed: i32) -> &mut Self {
        self.seed = Some(seed);

        self
    }
}

impl ProviderPayload for OpenRouterSettings {
    fn create(&self) -> Payload {
        let mut payload = Payload::new();

        payload.insert("model".to_string(), self.model.clone().into());
        payload.insert("messages".to_string(), serde_json::to_value(&self.prompt).unwrap());
        self.insert_option(&mut payload, "seed", self.seed);
        self.insert_option(&mut payload, "stream", self.stream);

        payload
    }
}

#[derive(Serialize, Deserialize)]
struct OpenRouterStreamEmpty {}

#[derive(Serialize, Deserialize)]
struct OpenRouterStreamChunk {
    id: String,
    choices: Vec<OpenRouterStreamChoice>,
    // Other fields...
}

#[derive(Serialize, Deserialize)]
struct OpenRouterStreamChoice {
    index: i32,
    delta: OpenRouterStreamDelta,
    finish_reason: Option<String>,
    native_finish_reason: Option<String>,
    lobprobs: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct OpenRouterStreamDelta {
    role: String,
    content: Option<String>
}

// To signal end of stream
struct OpenRouterStreamEnd {}

impl OpenRouterStreamChunk {
    fn update_with_complete_content(&mut self, complete_content: String) {
        // Depending on your struct design, update it to represent the complete content
        // This is just an example - adjust according to your actual implementation
        if !self.choices.is_empty() {
            // Create a new delta with the complete content
            self.choices[0].delta.content = Some(complete_content);
        }
    }
}

impl LLMProviderResponse for OpenRouterStreamEmpty {
    fn text(&self) -> String {
        String::new()
    }

    fn json(&self) -> serde_json::Value {
        serde_json::Value::Null
    }

    fn is_empty(&self) -> bool {
        true
    }
}

impl LLMProviderResponse for OpenRouterStreamChunk {
    fn text(&self) -> String {
        self.choices.get(0)
            .and_then(|choice| choice.delta.content.clone())
            .unwrap_or(String::new())
    }
    
    fn json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
    
    fn is_empty(&self) -> bool {
        self.choices.is_empty()
    }
}

impl LLMProviderResponse for OpenRouterStreamEnd {
    fn text(&self) -> String {
        "".to_string()
    }
    
    fn json(&self) -> serde_json::Value {
        serde_json::json!({ "done": true })
    }
    
    fn is_empty(&self) -> bool {
        true
    }
}