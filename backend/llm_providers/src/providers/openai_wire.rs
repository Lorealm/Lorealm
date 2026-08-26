//! The OpenAI chat-completions wire format, shared by every provider that
//! use it (OpenRouter, OpenAI, and the many local servers that
//! are OpenAI-compliant).

use crate::{error::*, types::*};
use serde::{Deserialize, Serialize};

fn role_str(role: &Role) -> &'static str {
    match role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// Convert domain messages to wire messages.
pub(super) fn to_wire_messages(messages: &[Message]) -> Vec<WireMessage> {
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

/// Convert advertised tool definitions to the wire `tools` array.
pub(super) fn to_wire_tools(tools: Option<&Vec<ToolDef>>) -> Option<Vec<WireTool>> {
    tools.map(|defs| {
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
    })
}

#[derive(Serialize)]
pub(super) struct WireMessage {
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
pub(super) struct WireTool {
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

#[derive(Deserialize)]
pub(super) struct ChatResponse {
    choices: Vec<ChatChoice>,
    usage: Option<WireUsage>,
    /// Which model actually served the request. Not every compatible server
    /// echoes it back, so it is optional here.
    #[serde(default)]
    model: String,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: Option<String>,
    /// Reasoning text. OpenRouter calls it `reasoning`; Some local
    /// servers (vLLM, llama.cpp) call it `reasoning_content`.
    reasoning: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<WireToolCall>>,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    total_tokens: u32,
}

/// Turn a parsed chat completion into the domain response.
pub(super) fn to_response(response: ChatResponse) -> Result<Response> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| Error::Provider("response contained no choices".to_string()))?;

    let mut content = Vec::new();
    if let Some(text) = choice
        .message
        .reasoning
        .or(choice.message.reasoning_content)
    {
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
        model: response.model,
        usage: response.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        }),
        finish_reason: choice.finish_reason,
    })
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: StreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning: Option<String>,
    reasoning_content: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Deserialize)]
struct StreamToolCall {
    index: Option<usize>,
    id: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Deserialize)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

/// Parse one SSE `data:` payload into a delta.
/// 
/// `Ok(None)` is to be treated as a termination token.
pub(super) fn to_delta(data: &str) -> Result<Option<Delta>> {
    if data == "[DONE]" {
        return Ok(None);
    }

    let chunk: StreamChunk = serde_json::from_str(data)?;

    let Some(choice) = chunk.choices.into_iter().next() else {
        return Ok(None);
    };

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
        reasoning: choice
            .delta
            .reasoning
            .or(choice.delta.reasoning_content)
            .unwrap_or_default(),
        tool_calls,
        finish_reason: choice.finish_reason,
    }))
}

/// Response format for `GET /v1/models`
#[derive(Deserialize)]
pub(super) struct ModelsResponse {
    data: Vec<WireModel>,
}

#[derive(Deserialize)]
struct WireModel {
    id: String,
    name: Option<String>,
    description: Option<String>,
    context_length: Option<u32>,
    pricing: Option<WireModelPricing>,
}

#[derive(Deserialize)]
struct WireModelPricing {
    prompt: Option<String>,
    completion: Option<String>,
}

pub(super) fn to_model_infos(response: ModelsResponse) -> Vec<ModelInfo> {
    response
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
        .collect()
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

    #[test]
    fn delta_accepts_either_reasoning_field() {
        let openrouter = to_delta(r#"{"choices":[{"delta":{"reasoning":"hmm"}}]}"#)
            .unwrap()
            .unwrap();
        assert_eq!(openrouter.reasoning, "hmm");

        let deepseek = to_delta(r#"{"choices":[{"delta":{"reasoning_content":"hmm"}}]}"#)
            .unwrap()
            .unwrap();
        assert_eq!(deepseek.reasoning, "hmm");

        assert!(to_delta("[DONE]").unwrap().is_none());
    }
}
