use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone)]
pub struct OpenRouterProvider {
    client: Arc<HttpClient>,
    api_key: String,
    base_url: String,
    model: String,
    request_modifier: Option<Arc<dyn Fn(RequestBuilder) -> RequestBuilder + Send + Sync>>,
}

#[derive(Debug, Clone, Default)]
pub struct OpenRouterOptions {
    pub provider: Option<OpenRouterProviderPreference>,
    pub transforms: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenRouterProviderPreference {
    pub order: Vec<String>,
    pub allow_fallbacks: Option<bool>,
}

impl OpenRouterProvider {
    pub fn new(
        client: Arc<HttpClient>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            client,
            api_key: api_key.into(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            model: model.into(),
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
struct OpenRouterRequest {
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
    provider: Option<OpenRouterProviderPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
    model: String,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: Message,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenRouterStreamResponse {
    choices: Vec<OpenRouterStreamChoice>,
}

#[derive(Deserialize)]
struct OpenRouterStreamChoice {
    delta: OpenRouterDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterDelta {
    content: Option<String>,
}

// Model listing types
#[derive(Deserialize)]
struct OpenRouterModelsResponse {
    data: Vec<OpenRouterModel>,
}

#[derive(Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<u32>,
    pricing: Option<OpenRouterModelPricing>,
}

#[derive(Deserialize)]
struct OpenRouterModelPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

#[async_trait]
impl super::LlmProvider for OpenRouterProvider {
    type ProviderParams = OpenRouterOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            provider: provider_params.provider.clone(),
            transforms: provider_params.transforms.clone(),
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

        let openrouter_response: OpenRouterResponse = response.json().await?;

        Ok(Response {
            content: openrouter_response.choices[0].message.content.clone(),
            model: openrouter_response.model,
            usage: openrouter_response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: openrouter_response.choices[0].finish_reason.clone(),
        })
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = OpenRouterRequest {
            model: self.model.clone(),
            messages,
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            provider: provider_params.provider.clone(),
            transforms: provider_params.transforms.clone(),
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

            let chunk: OpenRouterStreamResponse = serde_json::from_str(data)?;

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

    async fn list_models(&self) -> Result<Option<Vec<ModelInfo>>> {
        let builder = self
            .client
            .get(format!("{}/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key));

        let builder = self.apply_request_modifier(builder);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let models_response: OpenRouterModelsResponse = response.json().await?;

        let models = models_response
            .data
            .into_iter()
            .map(|m| ModelInfo {
                id: m.id,
                name: m.name,
                description: m.description,
                context_length: m.context_length,
                pricing: m.pricing.map(|p| ModelPricing {
                    prompt: p.prompt.and_then(|s| s.parse::<f64>().ok()),
                    completion: p.completion.and_then(|s| s.parse::<f64>().ok()),
                }),
            })
            .collect();

        Ok(Some(models))
    }
}
