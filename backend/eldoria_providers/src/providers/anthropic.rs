use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct AnthropicProvider {
    client: Arc<HttpClient>,
    api_key: String,
    base_url: String,
    request_modifier: Option<Arc<dyn Fn(RequestBuilder) -> RequestBuilder + Send + Sync>>,
}

#[derive(Debug, Clone, Default)]
pub struct AnthropicOptions {
    pub top_k: Option<u32>,
}

impl AnthropicProvider {
    pub fn new(client: Arc<HttpClient>, api_key: impl Into<String>) -> Self {
        Self {
            client,
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com/v1".to_string(),
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
struct AnthropicRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    model: String,
    usage: AnthropicUsage,
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamEvent {
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { delta: AnthropicDelta },
    #[serde(rename = "message_delta")]
    MessageDelta { delta: MessageDeltaData },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct AnthropicDelta {
    #[serde(default)]
    text: String,
}

#[derive(Deserialize)]
struct MessageDeltaData {
    stop_reason: Option<String>,
}

#[async_trait]
impl super::LlmProvider for AnthropicProvider {
    type ProviderParams = AnthropicOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        let request = AnthropicRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages,
            max_tokens: params.max_tokens.unwrap_or(1024),
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: provider_params.top_k,
            stop_sequences: params.stop.clone(),
            stream: None,
        };

        let builder = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request);

        let builder = self.apply_request_modifier(builder);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let anthropic_response: AnthropicResponse = response.json().await?;

        Ok(Response {
            content: anthropic_response.content[0].text.clone(),
            model: anthropic_response.model,
            usage: Some(Usage {
                prompt_tokens: anthropic_response.usage.input_tokens,
                completion_tokens: anthropic_response.usage.output_tokens,
                total_tokens: anthropic_response.usage.input_tokens
                    + anthropic_response.usage.output_tokens,
            }),
            finish_reason: anthropic_response.stop_reason,
        })
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = AnthropicRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages,
            max_tokens: params.max_tokens.unwrap_or(1024),
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: provider_params.top_k,
            stop_sequences: params.stop.clone(),
            stream: Some(true),
        };

        let builder = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request);

        let builder = self.apply_request_modifier(builder);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let stream = parse_sse_stream(response.bytes_stream(), |data| {
            let event: AnthropicStreamEvent = serde_json::from_str(data)?;

            match event {
                AnthropicStreamEvent::ContentBlockDelta { delta } => Ok(Some(Delta {
                    content: delta.text,
                    finish_reason: None,
                })),
                AnthropicStreamEvent::MessageDelta { delta } => Ok(Some(Delta {
                    content: String::new(),
                    finish_reason: delta.stop_reason,
                })),
                AnthropicStreamEvent::Other => Ok(None),
            }
        });

        Ok(Box::new(Box::pin(stream)))
    }
}