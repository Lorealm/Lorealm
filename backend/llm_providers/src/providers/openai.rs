//! Generic OpenAI chat-completions provider: OpenAI itself and anything that
//! mimics it (KoboldCPP, llama.cpp, vLLM, LM Studio, Ollama, TabbyAPI, ...).

use super::openai_wire::{self, WireMessage, WireTool};
use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::Stream;
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const OPENAI_DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

#[derive(Clone)]
pub struct OpenAiProvider {
    client: Arc<HttpClient>,
    api_key: Option<String>,
    base_url: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct OpenAiOptions {
    /// Root of the API, including any version segment, e.g.
    /// `http://localhost:5001/v1`. Defaults to [`OPENAI_DEFAULT_BASE_URL`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

impl OpenAiProvider {
    /// `base_url` is `None` for OpenAI itself; `api_key` is `None` for local
    /// servers that don't authenticate.
    pub fn new(
        client: Arc<HttpClient>,
        model: impl Into<String>,
        base_url: Option<String>,
        api_key: Option<String>,
    ) -> Self {
        let base_url = base_url.unwrap_or_else(|| OPENAI_DEFAULT_BASE_URL.to_string());
        Self {
            client,
            api_key,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.into(),
        }
    }

    /// Add the bearer header only when a key was configured: some local
    /// servers reject an `Authorization` header they didn't ask for.
    fn authorize(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.api_key {
            Some(key) => builder.header("Authorization", format!("Bearer {}", key)),
            None => builder,
        }
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        stream: bool,
    ) -> OpenAiRequest {
        OpenAiRequest {
            model: self.model.clone(),
            messages: openai_wire::to_wire_messages(&messages),
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            frequency_penalty: params.frequency_penalty,
            presence_penalty: params.presence_penalty,
            seed: params.seed,
            top_k: params.top_k,
            min_p: params.min_p,
            repetition_penalty: params.repetition_penalty,
            stop: params.stop.clone(),
            tools: openai_wire::to_wire_tools(params.tools.as_ref()),
            stream: if stream { Some(true) } else { None },
        }
    }
}

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<WireMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    // Not part of OpenAI's own schema, but widely accepted by local servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[async_trait]
impl super::LlmProvider for OpenAiProvider {
    type ProviderParams = OpenAiOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        _provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        let request = self.build_request(messages, params, false);

        let builder = self.authorize(
            self.client
                .post(format!("{}/chat/completions", self.base_url))
                .json(&request),
        );
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
        _provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = self.build_request(messages, params, true);

        let builder = self.authorize(
            self.client
                .post(format!("{}/chat/completions", self.base_url))
                .json(&request),
        );
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
        let builder = self.authorize(self.client.get(format!("{}/models", self.base_url)));
        let response = builder.send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let message = response.text().await.unwrap_or_default();
            return Err(Error::ApiError { status, message });
        }

        Ok(Some(openai_wire::to_model_infos(response.json().await?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider() -> OpenAiProvider {
        OpenAiProvider::new(
            Arc::new(HttpClient::new()),
            "koboldcpp/model",
            Some("http://localhost:5001/v1/".into()),
            None,
        )
    }

    #[test]
    fn trailing_slash_is_trimmed_from_base_url() {
        assert_eq!(provider().base_url, "http://localhost:5001/v1");
    }

    #[test]
    fn unset_params_are_omitted() {
        let request = provider().build_request(
            vec![Message::text(Role::User, "hi")],
            &CommonParams::default(),
            false,
        );

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "model": "koboldcpp/model",
                "messages": [{"role": "user", "content": "hi"}]
            })
        );
    }

    #[test]
    fn local_sampler_params_are_sent_when_set() {
        let params = CommonParams {
            temperature: Some(0.8),
            top_k: Some(40),
            min_p: Some(0.05),
            repetition_penalty: Some(1.1),
            ..CommonParams::default()
        };
        let request = provider().build_request(vec![], &params, true);

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "model": "koboldcpp/model",
                "messages": [],
                "temperature": 0.8_f32,
                "top_k": 40,
                "min_p": 0.05_f32,
                "repetition_penalty": 1.1_f32,
                "stream": true
            })
        );
    }
}
