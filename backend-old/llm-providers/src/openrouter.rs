use std::sync::{Arc, Mutex};

use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{parsers::sse::{SseEvent, SseStream}, samplers::{config::SamplerConfig, json::{Payload, ProviderPayload}, max_tokens::MaxTokensSampler, temperature::TemperatureSampler, top_p::TopPSampler, types::{Sampler, SamplerCollection}}};

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
    
    async fn generate_stream(
        self: Arc<Self>, 
        prompt: Vec<Message>
    ) -> Result<impl Stream<Item = Result<Box<dyn LLMProviderResponse>, Box<dyn std::error::Error + Send + Sync>>> + Send + Unpin, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Streaming with OpenRouter");
        
        let mut options = OpenRouterSettings::new(MODEL_NAME.to_string(), prompt)
            .with_stream(true);
        
        let config = SamplerConfig {
            temperature: Some((1.0, false, 0.0)),
            max_tokens: Some(200),
            ..Default::default()
        };
        
        let payload = options.with_samplers(&OpenRouterSamplers::samplers(&config));
        
        let client = reqwest::Client::new();
        let res = client
            .post(format!("{}/chat/completions", ENDPOINT))
            .json(&payload)
            .bearer_auth(API_KEY.to_string())
            .send()
            .await?;
        
        // Use a dedicated SSE parser
        let stream = SseStream::new(res.bytes_stream())
            .filter_map(|event| async move {
                match event {
                    Ok(SseEvent { data, .. }) => {
                        if data == "[DONE]" {
                            Some(Ok(Box::new(OpenRouterStreamEnd {}) as Box<dyn LLMProviderResponse>))
                        } else {
                            match serde_json::from_str::<OpenRouterStreamChunk>(&data) {
                                Ok(chunk) if !chunk.is_empty() => {
                                    Some(Ok(Box::new(chunk) as Box<dyn LLMProviderResponse>))
                                }
                                Ok(_) => None, // Empty chunk, filter out
                                Err(e) => {
                                    log::warn!("Failed to parse chunk: {}", e);
                                    None
                                }
                            }
                        }
                    }
                    Err(e) => Some(Err(e as Box<dyn std::error::Error + Send + Sync>)),
                }
            });
        
        Ok(Box::pin(stream))
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

    
    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);

        self
    }

    pub fn seed(mut self, seed: i32) -> Self {
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