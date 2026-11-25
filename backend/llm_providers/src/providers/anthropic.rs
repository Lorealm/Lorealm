//! Anthropic's Messages API (`POST /v1/messages`).

use super::anthropic_wire::{self, StreamParser, WireMessage, WireTool};
use super::openrouter::CacheControl;
use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const ANTHROPIC_DEFAULT_BASE_URL: &str = "https://api.anthropic.com/v1";

const ANTHROPIC_VERSION: &str = "2023-06-01";

/// The API requires `max_tokens`. Every current Claude model allows at
/// least this much output.
const DEFAULT_MAX_TOKENS: u32 = 16_000;

#[derive(Clone)]
pub struct AnthropicProvider {
    client: Arc<HttpClient>,
    api_key: String,
    base_url: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AnthropicOptions {
    /// Root of the API, including the version segment. Defaults to
    /// [`ANTHROPIC_DEFAULT_BASE_URL`]; set it to go through a proxy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<AnthropicThinking>,
    /// "low" / "medium" / "high" / "xhigh" / "max". Sent as
    /// `output_config.effort`; which levels exist depends on the model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
}

/// The `thinking` request parameter, serialized as-is. Current models take
/// `Adaptive`; `Enabled` with a fixed budget is for older models only.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AnthropicThinking {
    Adaptive {
        /// "summarized" returns readable reasoning; recent models default to
        /// "omitted", which streams thinking blocks with empty text.
        #[serde(skip_serializing_if = "Option::is_none")]
        display: Option<String>,
    },
    Enabled {
        budget_tokens: u32,
    },
    Disabled,
}

impl AnthropicProvider {
    /// `base_url` is `None` for Anthropic itself.
    pub fn new(
        client: Arc<HttpClient>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: impl Into<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| ANTHROPIC_DEFAULT_BASE_URL.to_string());
        Self {
            client,
            api_key: api_key.into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
        }
    }

    fn authorize(&self, builder: RequestBuilder) -> RequestBuilder {
        builder
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        options: &AnthropicOptions,
        stream: bool,
    ) -> AnthropicRequest {
        let (system, messages) = anthropic_wire::to_wire_messages(&messages);
        // Penalties, min_p and seed have no Messages API equivalent and are
        // dropped. Recent models also reject temperature/top_p/top_k with a
        // 400, so those are only sent when explicitly set.
        AnthropicRequest {
            model: self.model.clone(),
            max_tokens: params.max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            cache_control: CacheControl::ephemeral(),
            system,
            messages,
            temperature: params.temperature,
            top_p: params.top_p,
            top_k: params.top_k,
            stop_sequences: params.stop.clone(),
            tools: anthropic_wire::to_wire_tools(params.tools.as_ref()),
            thinking: options.thinking.clone(),
            output_config: options.effort.clone().map(|effort| OutputConfig { effort }),
            stream: if stream { Some(true) } else { None },
        }
    }

    async fn post(
        &self,
        request: &AnthropicRequest,
        params: &CommonParams,
    ) -> Result<reqwest::Response> {
        let builder = super::with_custom_fields(
            self.authorize(self.client.post(format!("{}/messages", self.base_url))),
            request,
            params,
        );
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }
        Ok(response)
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    /// Automatic prompt caching: the API places the breakpoint on the last
    /// cacheable block, so a growing chat reuses its prefix each turn.
    cache_control: CacheControl,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<AnthropicThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_config: Option<OutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Serialize)]
struct OutputConfig {
    effort: String,
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
        let request = self.build_request(messages, params, provider_params, false);
        let response = self.post(&request, params).await?;
        Ok(anthropic_wire::to_response(response.json().await?))
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = self.build_request(messages, params, provider_params, true);
        let response = self.post(&request, params).await?;

        let mut parser = StreamParser::default();
        let stream = parse_sse_stream(response.bytes_stream(), move |data| parser.parse(data));

        Ok(Box::new(Box::pin(stream)))
    }

    async fn list_models(&self) -> Result<Option<Vec<ModelInfo>>> {
        // One page of the maximum size covers the whole catalogue.
        let builder = self.authorize(
            self.client
                .get(format!("{}/models?limit=1000", self.base_url)),
        );
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        Ok(Some(anthropic_wire::to_model_infos(response.json().await?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> AnthropicProvider {
        AnthropicProvider::new(
            Arc::new(HttpClient::new()),
            "claude-opus-5",
            Some("https://proxy.example/v1/".into()),
            "key",
        )
    }

    #[test]
    fn trailing_slash_is_trimmed_from_base_url() {
        assert_eq!(provider().base_url, "https://proxy.example/v1");
    }

    #[test]
    fn minimal_request_has_required_fields_only() {
        let request = provider().build_request(
            vec![Message::text(Role::User, "hi")],
            &CommonParams::default(),
            &AnthropicOptions::default(),
            false,
        );

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "model": "claude-opus-5",
                "max_tokens": 16000,
                "cache_control": {"type": "ephemeral"},
                "messages": [{"role": "user", "content": [{"type": "text", "text": "hi"}]}]
            })
        );
    }

    #[test]
    fn options_and_supported_params_are_sent() {
        let params = CommonParams {
            max_tokens: Some(1024),
            stop: Some(vec!["END".into()]),
            // No Messages API equivalent: dropped.
            frequency_penalty: Some(0.5),
            seed: Some(7),
            ..CommonParams::default()
        };
        let options: AnthropicOptions = serde_json::from_value(serde_json::json!({
            "thinking": {"type": "adaptive", "display": "summarized"},
            "effort": "high"
        }))
        .unwrap();
        let request = provider().build_request(
            vec![
                Message::text(Role::System, "Be terse"),
                Message::text(Role::User, "hi"),
            ],
            &params,
            &options,
            true,
        );

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "model": "claude-opus-5",
                "max_tokens": 1024,
                "cache_control": {"type": "ephemeral"},
                "system": "Be terse",
                "messages": [{"role": "user", "content": [{"type": "text", "text": "hi"}]}],
                "stop_sequences": ["END"],
                "thinking": {"type": "adaptive", "display": "summarized"},
                "output_config": {"effort": "high"},
                "stream": true
            })
        );
    }

    #[test]
    fn unknown_option_fields_are_rejected() {
        let result: std::result::Result<AnthropicOptions, _> =
            serde_json::from_value(serde_json::json!({"thinkng": {"type": "adaptive"}}));
        assert!(result.is_err());
    }
}
