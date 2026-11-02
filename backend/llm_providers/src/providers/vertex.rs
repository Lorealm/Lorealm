//! Gemini models on Google Cloud Vertex AI (`generateContent`).

use super::gemini_wire::{self, DeltaParser, WireContent, WireTool};
use crate::{error::*, streaming::parse_sse_stream, types::*};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use reqwest::{Client as HttpClient, RequestBuilder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const VERTEX_DEFAULT_LOCATION: &str = "global";

/// How requests authenticate.
#[derive(Clone)]
pub enum VertexAuth {
    /// A Vertex AI API key. Works with our without a project.
    ApiKey(String),
    /// An OAuth2 access token, e.g. from `gcloud auth print-access-token`.
    /// These expire (typically after an hour); refreshing them is up to the
    /// caller. Requires a project.
    AccessToken(String),
}

#[derive(Clone)]
pub struct VertexProvider {
    client: Arc<HttpClient>,
    auth: VertexAuth,
    base_url: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct VertexOptions {
    /// Google Cloud project id. Leave unset for express mode (API key only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// e.g. "us-central1". Defaults to [`VERTEX_DEFAULT_LOCATION`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<VertexThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_settings: Option<Vec<VertexSafetySetting>>,
}

/// Gemini's `thinkingConfig`. Gemini 3 models take `thinking_level`
/// ("low" / "high"); Gemini 2.5 models take `thinking_budget` in tokens
/// (0 disables thinking where allowed, -1 lets the model decide).
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields, default)]
pub struct VertexThinking {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budget: Option<i32>,
    /// Return thought summaries alongside the answer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_thoughts: Option<bool>,
}

/// One entry of `safetySettings`, e.g. category
/// "HARM_CATEGORY_DANGEROUS_CONTENT" with threshold "BLOCK_ONLY_HIGH".
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct VertexSafetySetting {
    pub category: String,
    pub threshold: String,
}

impl VertexProvider {
    /// `project_id` is `None` for express mode, which only accepts API keys.
    pub fn new(
        client: Arc<HttpClient>,
        model: impl Into<String>,
        auth: VertexAuth,
        project_id: Option<String>,
        location: Option<String>,
    ) -> Self {
        let location = location.unwrap_or_else(|| VERTEX_DEFAULT_LOCATION.to_string());
        let host = if location == "global" {
            "https://aiplatform.googleapis.com".to_string()
        } else {
            format!("https://{}-aiplatform.googleapis.com", location)
        };
        let base_url = match project_id {
            Some(project) => format!(
                "{}/v1/projects/{}/locations/{}/publishers/google",
                host, project, location
            ),
            None => format!("{}/v1/publishers/google", host),
        };
        Self {
            client,
            auth,
            base_url,
            model: model.into(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_string();
        self
    }

    fn authorize(&self, builder: RequestBuilder) -> RequestBuilder {
        match &self.auth {
            // A header rather than `?key=` keeps the key out of URL logs.
            VertexAuth::ApiKey(key) => builder.header("x-goog-api-key", key),
            VertexAuth::AccessToken(token) => {
                builder.header("Authorization", format!("Bearer {}", token))
            }
        }
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        options: &VertexOptions,
    ) -> GenerateRequest {
        let (system_instruction, contents) = gemini_wire::to_wire_contents(&messages);
        // min_p and repetition_penalty have no Gemini equivalent and are dropped.
        GenerateRequest {
            contents,
            system_instruction,
            tools: gemini_wire::to_wire_tools(params.tools.as_ref()),
            generation_config: GenerationConfig {
                temperature: params.temperature,
                top_p: params.top_p,
                top_k: params.top_k,
                max_output_tokens: params.max_tokens,
                stop_sequences: params.stop.clone(),
                presence_penalty: params.presence_penalty,
                frequency_penalty: params.frequency_penalty,
                seed: params.seed,
                thinking_config: options.thinking.as_ref().map(|t| ThinkingConfig {
                    thinking_level: t.thinking_level.clone(),
                    thinking_budget: t.thinking_budget,
                    include_thoughts: t.include_thoughts,
                }),
            },
            safety_settings: options.safety_settings.clone(),
        }
    }

    async fn post(&self, method: &str, request: &GenerateRequest) -> Result<reqwest::Response> {
        let url = format!("{}/models/{}:{}", self.base_url, self.model, method);
        let builder = self.authorize(self.client.post(url).json(request));
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
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    contents: Vec<WireContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<WireContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<WireTool>>,
    generation_config: GenerationConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    safety_settings: Option<Vec<VertexSafetySetting>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_config: Option<ThinkingConfig>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ThinkingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking_budget: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_thoughts: Option<bool>,
}

#[async_trait]
impl super::LlmProvider for VertexProvider {
    type ProviderParams = VertexOptions;

    async fn complete(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Response> {
        let request = self.build_request(messages, params, provider_params);
        let response = self.post("generateContent", &request).await?;
        gemini_wire::to_response(response.json().await?)
    }

    async fn stream(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &Self::ProviderParams,
    ) -> Result<Box<dyn Stream<Item = Result<Delta>> + Unpin + Send>> {
        let request = self.build_request(messages, params, provider_params);
        let response = self.post("streamGenerateContent?alt=sse", &request).await?;

        let mut parser = DeltaParser::default();
        let stream = parse_sse_stream(response.bytes_stream(), move |data| parser.parse(data))
            .flat_map(|chunk| {
                let deltas: Vec<Result<Delta>> = match chunk {
                    Ok(deltas) => deltas.into_iter().map(Ok).collect(),
                    Err(e) => vec![Err(e)],
                };
                futures::stream::iter(deltas)
            });

        Ok(Box::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(project: Option<&str>, location: Option<&str>) -> VertexProvider {
        VertexProvider::new(
            Arc::new(HttpClient::new()),
            "gemini-3-pro",
            VertexAuth::ApiKey("key".into()),
            project.map(Into::into),
            location.map(Into::into),
        )
    }

    #[test]
    fn endpoint_follows_project_and_location() {
        assert_eq!(
            provider(None, None).base_url,
            "https://aiplatform.googleapis.com/v1/publishers/google"
        );
        assert_eq!(
            provider(Some("proj"), None).base_url,
            "https://aiplatform.googleapis.com/v1/projects/proj/locations/global/publishers/google"
        );
        assert_eq!(
            provider(Some("proj"), Some("europe-west4")).base_url,
            "https://europe-west4-aiplatform.googleapis.com/v1/projects/proj/locations/europe-west4/publishers/google"
        );
    }

    #[test]
    fn request_maps_params_and_options() {
        let params = CommonParams {
            temperature: Some(0.5),
            max_tokens: Some(512),
            top_k: Some(40),
            min_p: Some(0.1),
            tools: Some(vec![ToolDef {
                name: "add".into(),
                description: "Add".into(),
                input_schema: serde_json::json!({"type": "object"}),
            }]),
            ..CommonParams::default()
        };
        let options: VertexOptions = serde_json::from_value(serde_json::json!({
            "thinking": {"thinking_level": "low", "include_thoughts": true},
            "safety_settings": [{"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"}]
        }))
        .unwrap();
        let request = provider(None, None).build_request(
            vec![
                Message::text(Role::System, "Be terse"),
                Message::text(Role::User, "hi"),
            ],
            &params,
            &options,
        );

        assert_eq!(
            serde_json::to_value(&request).unwrap(),
            serde_json::json!({
                "contents": [{"role": "user", "parts": [{"text": "hi"}]}],
                "systemInstruction": {"parts": [{"text": "Be terse"}]},
                "tools": [{"functionDeclarations": [
                    {"name": "add", "description": "Add", "parametersJsonSchema": {"type": "object"}}
                ]}],
                "generationConfig": {
                    "temperature": 0.5,
                    "topK": 40,
                    "maxOutputTokens": 512,
                    "thinkingConfig": {"thinkingLevel": "low", "includeThoughts": true}
                },
                "safetySettings": [{"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"}]
            })
        );
    }
}
