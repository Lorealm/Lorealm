use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct OpenAIProvider {
    client: Arc<HttpClient>,
    api_key: String,
    base_url: String,
    request_modifier: Option<Arc<dyn Fn(RequestBuilder) -> RequestBuilder + Send + Sync>>,
}

#[derive(Debug, Clone, Default)]
pub struct OpenAIOptions {
    pub seed: Option<u32>,
    pub response_format: Option<ResponseFormat>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "text")]
    Text,
}

impl OpenAIProvider {
    pub fn new(client: Arc<HttpClient>, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            request_modifier: None,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_request_modifier<F>(mut self, modifier: F) -> Self
    where
        F: Fn(RequestBuilder) -> RequestBuilder + Send + Sync + 'static,
    {
        self.request_modifier = Some(Arc::new(modifier));
        self
    }

    fn apply_request_modifier(&self, builder: RequestBuilder) -> RequestBuilder {
        if let Some(modifier) = &self.request_modifier {
            modifier(builder)
        } else {
            builder
        }
    }
}

#[derive(Serialize)]
struct OpenAIRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<ResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
    usage: OpenAIUsage,
    model: String,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAIStreamResponse {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIDelta {
    content: Option<String>,
}

#[async_trait]
impl super::LlmProvider for OpenAIProvider {
    type ProviderParams = OpenAIOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            seed: provider_params.seed,
            response_format: provider_params.response_format.clone(),
            frequency_penalty: provider_params.frequency_penalty,
            presence_penalty: provider_params.presence_penalty,
            stream: None,
        };

        let builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request);

        let builder = self.apply_request_modifier(builder);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let openai_response: OpenAIResponse = response.json().await?;

        Ok(Response {
            content: openai_response.choices[0].message.content.clone(),
            model: openai_response.model,
            usage: Some(Usage {
                prompt_tokens: openai_response.usage.prompt_tokens,
                completion_tokens: openai_response.usage.completion_tokens,
                total_tokens: openai_response.usage.total_tokens,
            }),
            finish_reason: openai_response.choices[0].finish_reason.clone(),
        })
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = OpenAIRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            seed: provider_params.seed,
            response_format: provider_params.response_format.clone(),
            frequency_penalty: provider_params.frequency_penalty,
            presence_penalty: provider_params.presence_penalty,
            stream: Some(true),
        };

        let builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request);

        let builder = self.apply_request_modifier(builder);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let stream = parse_sse_stream(response.bytes_stream(), |data| {
            if data == "[DONE]" {
                return Ok(None);
            }

            let chunk: OpenAIStreamResponse = serde_json::from_str(data)?;
            
            if let Some(choice) = chunk.choices.first() {
                Ok(Some(Delta {
                    content: choice.delta.content.clone().unwrap_or_default(),
                    finish_reason: choice.finish_reason.clone(),
                }))
            } else {
                Ok(None)
            }
        });

        Ok(Box::new(Box::pin(stream)))
    }
}