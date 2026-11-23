use super::openai_wire::{self, WireMessage, WireTool};
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

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct OpenRouterOptions {
    pub provider: Option<OpenRouterProviderPreference>,
    pub transforms: Option<Vec<String>>,
    pub reasoning: Option<OpenRouterReasoning>,
}

/// OpenRouter's normalized reasoning configuration. Effort is one of
/// "low" / "medium" / "high"; some models take a token budget instead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenRouterReasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
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

    fn build_request(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &OpenRouterOptions,
        stream: bool,
    ) -> OpenRouterRequest {
        OpenRouterRequest {
            model: self.model.clone(),
            cache_control: CacheControl::ephemeral(),
            messages: openai_wire::to_wire_messages(&messages),
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            tools: openai_wire::to_wire_tools(params.tools.as_ref()),
            provider: provider_params.provider.clone(),
            transforms: provider_params.transforms.clone(),
            reasoning: provider_params.reasoning.clone(),
            stream: if stream { Some(true) } else { None },
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    Ephemeral,
}

#[derive(Serialize)]
pub struct CacheControl {
    r#type: CacheType,
}

impl CacheControl {
    pub fn ephemeral() -> Self {
        Self {
            r#type: CacheType::Ephemeral,
        }
    }
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    cache_control: CacheControl,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<OpenRouterProviderPreference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transforms: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<OpenRouterReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
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
        let request = self.build_request(messages, params, provider_params, false);

        let builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key));

        let builder =
            super::with_custom_fields(self.apply_request_modifier(builder), &request, params);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        openai_wire::to_response(response.json().await?)
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = self.build_request(messages, params, provider_params, true);

        let builder = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key));

        let builder =
            super::with_custom_fields(self.apply_request_modifier(builder), &request, params);
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        let stream = parse_sse_stream(response.bytes_stream(), openai_wire::to_delta);

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

        Ok(Some(openai_wire::to_model_infos(response.json().await?)))
    }
}
