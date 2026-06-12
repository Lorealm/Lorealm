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
    pub reasoning: Option<OpenRouterReasoning>,
}

/// OpenRouter's normalized reasoning configuration. Effort is one of
/// "low" / "medium" / "high"; some models take a token budget instead.
#[derive(Debug, Clone, Serialize)]
pub struct OpenRouterReasoning {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
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

    fn build_request(
        &self,
        messages: Vec<Message>,
        params: &CommonParams,
        provider_params: &OpenRouterOptions,
        stream: bool,
    ) -> OpenRouterRequest {
        OpenRouterRequest {
            model: self.model.clone(),
            messages: to_wire_messages(&messages),
            temperature: params.temperature,
            max_tokens: params.max_tokens,
            top_p: params.top_p,
            stop: params.stop.clone(),
            tools: params.tools.as_ref().map(|defs| {
                defs.iter()
                    .map(|d| WireTool {
                        kind: "function",
                        function: WireFunctionDef {
                            name: d.name.clone(),
                            description: d.description.clone(),
                            parameters: d.input_schema.clone(),
                        },
                    })
                    .collect()
            }),
            provider: provider_params.provider.clone(),
            transforms: provider_params.transforms.clone(),
            reasoning: provider_params.reasoning.clone(),
            stream: if stream { Some(true) } else { None },
        }
    }
}

fn role_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// Convert domain messages to wire messages. One domain message can expand to
/// several wire messages: OpenAI-style APIs expect each tool result as its own
/// `role: "tool"` message, while our model keeps them as blocks.
fn to_wire_messages(messages: &[Message]) -> Vec<WireMessage> {
    let mut wire = Vec::new();
    for msg in messages {
        let mut parts = Vec::new();
        let mut tool_calls = Vec::new();
        for block in &msg.content {
            match block {
                ContentBlock::Text { text } => parts.push(WirePart::Text { text: text.clone() }),
                ContentBlock::Image { media_type, source } => {
                    let url = match source {
                        ImageSource::Url(url) => url.clone(),
                        ImageSource::Base64(data) => {
                            format!("data:{};base64,{}", media_type, data)
                        }
                    };
                    parts.push(WirePart::ImageUrl {
                        image_url: WireImageUrl { url },
                    });
                }
                // Reasoning is model-internal output; there is no slot for it
                // in the request format, so it is never sent back.
                ContentBlock::Thinking { .. } => {}
                ContentBlock::ToolUse { id, name, input } => tool_calls.push(WireToolCall {
                    id: id.clone(),
                    kind: "function".to_string(),
                    function: WireFunctionCall {
                        name: name.clone(),
                        arguments: input.to_string(),
                    },
                }),
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => {
                    let content = if *is_error {
                        format!("Error: {}", content)
                    } else {
                        content.clone()
                    };
                    wire.push(WireMessage {
                        role: "tool",
                        content: Some(WireContent::Text(content)),
                        tool_calls: None,
                        tool_call_id: Some(tool_use_id.clone()),
                    });
                }
            }
        }

        if parts.is_empty() && tool_calls.is_empty() {
            continue;
        }
        // A single text block serializes as a plain string for maximum
        // compatibility; anything else uses the multi-part array form.
        let mut parts = parts;
        let content = if parts.is_empty() {
            None
        } else if parts.len() == 1 {
            match parts.pop().unwrap() {
                WirePart::Text { text } => Some(WireContent::Text(text)),
                part => Some(WireContent::Parts(vec![part])),
            }
        } else {
            Some(WireContent::Parts(parts))
        };
        wire.push(WireMessage {
            role: role_str(&msg.role),
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
        });
    }
    wire
}

#[derive(Serialize)]
struct WireMessage {
    role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<WireContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<WireToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum WireContent {
    Text(String),
    Parts(Vec<WirePart>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WirePart {
    Text { text: String },
    ImageUrl { image_url: WireImageUrl },
}

#[derive(Serialize)]
struct WireImageUrl {
    url: String,
}

#[derive(Serialize, Deserialize)]
struct WireToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    function: WireFunctionCall,
}

#[derive(Serialize, Deserialize)]
struct WireFunctionCall {
    name: String,
    /// JSON-encoded arguments object, per the OpenAI wire format.
    arguments: String,
}

#[derive(Serialize)]
struct WireTool {
    #[serde(rename = "type")]
    kind: &'static str,
    function: WireFunctionDef,
}

#[derive(Serialize)]
struct WireFunctionDef {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
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

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
    model: String,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenRouterResponseMessage {
    content: Option<String>,
    reasoning: Option<String>,
    tool_calls: Option<Vec<WireToolCall>>,
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
    reasoning: Option<String>,
    tool_calls: Option<Vec<OpenRouterStreamToolCall>>,
}

#[derive(Deserialize)]
struct OpenRouterStreamToolCall {
    index: Option<usize>,
    id: Option<String>,
    function: Option<OpenRouterStreamFunction>,
}

#[derive(Deserialize)]
struct OpenRouterStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
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
        let request = self.build_request(messages, params, provider_params, false);

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

        let choice = openrouter_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| Error::Provider("response contained no choices".to_string()))?;

        let mut content = Vec::new();
        if let Some(text) = choice.message.reasoning {
            if !text.is_empty() {
                content.push(ContentBlock::Thinking { text });
            }
        }
        if let Some(text) = choice.message.content {
            if !text.is_empty() {
                content.push(ContentBlock::Text { text });
            }
        }
        for call in choice.message.tool_calls.unwrap_or_default() {
            let input = if call.function.arguments.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::from_str(&call.function.arguments)?
            };
            content.push(ContentBlock::ToolUse {
                id: call.id,
                name: call.function.name,
                input,
            });
        }

        Ok(Response {
            content,
            model: openrouter_response.model,
            usage: openrouter_response.usage.map(|u| Usage {
                prompt_tokens: u.prompt_tokens,
                completion_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
            }),
            finish_reason: choice.finish_reason,
        })
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

            if let Some(choice) = chunk.choices.into_iter().next() {
                let tool_calls = choice
                    .delta
                    .tool_calls
                    .unwrap_or_default()
                    .into_iter()
                    .map(|tc| ToolCallDelta {
                        index: tc.index.unwrap_or(0),
                        id: tc.id,
                        name: tc.function.as_ref().and_then(|f| f.name.clone()),
                        arguments: tc.function.and_then(|f| f.arguments).unwrap_or_default(),
                    })
                    .collect();

                Ok(Some(Delta {
                    content: choice.delta.content.unwrap_or_default(),
                    reasoning: choice.delta.reasoning.unwrap_or_default(),
                    tool_calls,
                    finish_reason: choice.finish_reason,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_conversion_matches_openai_format() {
        let messages = vec![
            Message::text(Role::System, "Be helpful"),
            Message {
                role: Role::User,
                content: vec![
                    ContentBlock::Text {
                        text: "What is in this image?".into(),
                    },
                    ContentBlock::Image {
                        media_type: "image/png".into(),
                        source: ImageSource::Base64("AAAA".into()),
                    },
                ],
            },
            Message {
                role: Role::Assistant,
                content: vec![
                    ContentBlock::Thinking {
                        text: "I should add these.".into(),
                    },
                    ContentBlock::ToolUse {
                        id: "call_1".into(),
                        name: "add".into(),
                        input: serde_json::json!({"a": 1}),
                    },
                ],
            },
            Message {
                role: Role::User,
                content: vec![ContentBlock::ToolResult {
                    tool_use_id: "call_1".into(),
                    content: "2".into(),
                    is_error: false,
                }],
            },
        ];

        let wire = serde_json::to_value(to_wire_messages(&messages)).unwrap();
        assert_eq!(
            wire,
            serde_json::json!([
                {"role": "system", "content": "Be helpful"},
                {"role": "user", "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}
                ]},
                {"role": "assistant", "tool_calls": [
                    {"id": "call_1", "type": "function",
                     "function": {"name": "add", "arguments": "{\"a\":1}"}}
                ]},
                {"role": "tool", "content": "2", "tool_call_id": "call_1"}
            ])
        );
    }
}
