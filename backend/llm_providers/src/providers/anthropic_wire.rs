//! The Anthropic Messages wire format. Kept apart from the provider so other
//! platforms that serve Claude with the same body (e.g. Vertex AI's
//! `rawPredict`) can reuse it.

use crate::{error::*, types::*};
use serde::{Deserialize, Serialize};

/// Convert domain messages to the top-level `system` text plus wire
/// messages.
///
/// Leading system messages become `system`. Not every Claude model accepts
/// a mid-conversation system role, so later system messages are sent as user
/// text. Consecutive same-role messages are merged into one.
pub(super) fn to_wire_messages(messages: &[Message]) -> (Option<String>, Vec<WireMessage>) {
    let leading = messages
        .iter()
        .take_while(|m| matches!(m.role, Role::System))
        .count();
    let system: Vec<String> = messages[..leading]
        .iter()
        .flat_map(|m| &m.content)
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(text.clone()),
            _ => None,
        })
        .collect();
    let system = if system.is_empty() {
        None
    } else {
        Some(system.join("\n\n"))
    };

    let mut wire: Vec<WireMessage> = Vec::new();
    for msg in &messages[leading..] {
        let role = match msg.role {
            Role::Assistant => "assistant",
            Role::User | Role::System => "user",
        };
        let content: Vec<WireBlock> = msg.content.iter().filter_map(to_wire_block).collect();
        if content.is_empty() {
            continue;
        }
        match wire.last_mut() {
            Some(last) if last.role == role => last.content.extend(content),
            _ => wire.push(WireMessage { role, content }),
        }
    }
    (system, wire)
}

fn to_wire_block(block: &ContentBlock) -> Option<WireBlock> {
    Some(match block {
        ContentBlock::Text { text } => WireBlock::Text { text: text.clone() },
        ContentBlock::Image { media_type, source } => WireBlock::Image {
            source: match source {
                ImageSource::Base64(data) => WireImageSource::Base64 {
                    media_type: media_type.clone(),
                    data: data.clone(),
                },
                ImageSource::Url(url) => WireImageSource::Url { url: url.clone() },
            },
        },
        // Only signed blocks can be replayed; the API rejects thinking it
        // can't verify, so unsigned reasoning (e.g. from another provider)
        // is dropped.
        ContentBlock::Thinking { text, signature } => WireBlock::Thinking {
            thinking: text.clone(),
            signature: signature.clone()?,
        },
        ContentBlock::ToolUse { id, name, input } => WireBlock::ToolUse {
            id: id.clone(),
            name: name.clone(),
            // A call with no arguments collects to `null`; the API requires
            // an object.
            input: if input.is_null() {
                serde_json::json!({})
            } else {
                input.clone()
            },
        },
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } => WireBlock::ToolResult {
            tool_use_id: tool_use_id.clone(),
            content: content.clone(),
            is_error: *is_error,
        },
    })
}

/// Convert advertised tool definitions to the wire `tools` array.
pub(super) fn to_wire_tools(tools: Option<&Vec<ToolDef>>) -> Option<Vec<WireTool>> {
    tools.map(|defs| {
        defs.iter()
            .map(|d| WireTool {
                name: d.name.clone(),
                description: d.description.clone(),
                input_schema: d.input_schema.clone(),
            })
            .collect()
    })
}

#[derive(Serialize)]
pub(super) struct WireMessage {
    role: &'static str,
    content: Vec<WireBlock>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireBlock {
    Text {
        text: String,
    },
    Image {
        source: WireImageSource,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        is_error: bool,
    },
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireImageSource {
    Base64 { media_type: String, data: String },
    Url { url: String },
}

#[derive(Serialize)]
pub(super) struct WireTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Deserialize)]
pub(super) struct MessagesResponse {
    model: String,
    content: Vec<ResponseBlock>,
    stop_reason: Option<String>,
    usage: Option<WireUsage>,
}

/// Response content blocks. Server-tool results, `redacted_thinking` and
/// anything newer have no domain counterpart and are skipped.
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ResponseBlock {
    Text {
        text: String,
    },
    Thinking {
        #[serde(default)]
        thinking: String,
        #[serde(default)]
        signature: String,
    },
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct WireUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    cache_creation_input_tokens: u32,
    #[serde(default)]
    cache_read_input_tokens: u32,
}

fn non_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// Turn a parsed Messages response into the domain response.
pub(super) fn to_response(response: MessagesResponse) -> Response {
    let content = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            ResponseBlock::Text { text } => Some(ContentBlock::Text { text }),
            ResponseBlock::Thinking {
                thinking,
                signature,
            } => Some(ContentBlock::Thinking {
                text: thinking,
                signature: non_empty(signature),
            }),
            ResponseBlock::ToolUse { id, name, input } => {
                Some(ContentBlock::ToolUse { id, name, input })
            }
            ResponseBlock::Other => None,
        })
        .collect();

    Response {
        content,
        model: response.model,
        usage: response.usage.map(|u| {
            // Cached input is billed separately but is still prompt input.
            let prompt = u.input_tokens + u.cache_creation_input_tokens + u.cache_read_input_tokens;
            Usage {
                prompt_tokens: prompt,
                completion_tokens: u.output_tokens,
                total_tokens: prompt + u.output_tokens,
            }
        }),
        finish_reason: response.stop_reason,
    }
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum StreamEvent {
    ContentBlockStart {
        index: usize,
        content_block: ResponseBlock,
    },
    ContentBlockDelta {
        index: usize,
        delta: BlockDelta,
    },
    MessageDelta {
        delta: MessageDeltaBody,
    },
    Error {
        error: WireError,
    },
    /// `message_start`, `content_block_stop`, `message_stop`, `ping`.
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BlockDelta {
    TextDelta {
        text: String,
    },
    ThinkingDelta {
        thinking: String,
    },
    SignatureDelta {
        signature: String,
    },
    InputJsonDelta {
        partial_json: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Deserialize)]
struct MessageDeltaBody {
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct WireError {
    #[serde(rename = "type")]
    kind: String,
    message: String,
}

/// Per-stream parser state. Content block indices count every block (text,
/// thinking, tool use), but [`ToolCallDelta::index`] must count tool calls
/// only, so the mapping is tracked here.
#[derive(Default)]
pub(super) struct StreamParser {
    /// (content block index, tool call index)
    tool_indices: Vec<(usize, usize)>,
}

impl StreamParser {
    /// Parse one SSE `data:` payload. `Ok(None)` means nothing to emit.
    pub(super) fn parse(&mut self, data: &str) -> Result<Option<Delta>> {
        let event: StreamEvent = serde_json::from_str(data)?;
        let delta = match event {
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => match content_block {
                ResponseBlock::Text { text } => Delta {
                    content: text,
                    ..Delta::default()
                },
                ResponseBlock::Thinking {
                    thinking,
                    signature,
                } => Delta {
                    reasoning: thinking,
                    reasoning_signature: non_empty(signature),
                    ..Delta::default()
                },
                // `input` is always `{}` here; the arguments follow as
                // `input_json_delta` fragments.
                ResponseBlock::ToolUse { id, name, .. } => {
                    let call = self.tool_indices.len();
                    self.tool_indices.push((index, call));
                    Delta {
                        tool_calls: vec![ToolCallDelta {
                            index: call,
                            id: Some(id),
                            name: Some(name),
                            arguments: String::new(),
                        }],
                        ..Delta::default()
                    }
                }
                ResponseBlock::Other => return Ok(None),
            },
            StreamEvent::ContentBlockDelta { index, delta } => match delta {
                BlockDelta::TextDelta { text } => Delta {
                    content: text,
                    ..Delta::default()
                },
                BlockDelta::ThinkingDelta { thinking } => Delta {
                    reasoning: thinking,
                    ..Delta::default()
                },
                BlockDelta::SignatureDelta { signature } => Delta {
                    reasoning_signature: Some(signature),
                    ..Delta::default()
                },
                BlockDelta::InputJsonDelta { partial_json } => {
                    let Some(&(_, call)) = self.tool_indices.iter().find(|(i, _)| *i == index)
                    else {
                        return Err(Error::StreamError(format!(
                            "input_json_delta for unknown block {}",
                            index
                        )));
                    };
                    Delta {
                        tool_calls: vec![ToolCallDelta {
                            index: call,
                            id: None,
                            name: None,
                            arguments: partial_json,
                        }],
                        ..Delta::default()
                    }
                }
                BlockDelta::Other => return Ok(None),
            },
            StreamEvent::MessageDelta { delta } => Delta {
                finish_reason: delta.stop_reason,
                ..Delta::default()
            },
            // Mid-stream failures (e.g. `overloaded_error`) arrive as events
            // after a 200 response.
            StreamEvent::Error { error } => {
                return Err(Error::StreamError(format!(
                    "{}: {}",
                    error.kind, error.message
                )));
            }
            StreamEvent::Other => return Ok(None),
        };
        Ok(Some(delta))
    }
}

/// Response format for `GET /v1/models`
#[derive(Deserialize)]
pub(super) struct ModelsResponse {
    data: Vec<WireModel>,
}

#[derive(Deserialize)]
struct WireModel {
    id: String,
    display_name: Option<String>,
    max_input_tokens: Option<u32>,
}

pub(super) fn to_model_infos(response: ModelsResponse) -> Vec<ModelInfo> {
    response
        .data
        .into_iter()
        .map(|m| ModelInfo {
            id: m.id,
            name: m.display_name,
            description: None,
            context_length: m.max_input_tokens,
            pricing: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::streaming::BlockAccumulator;

    #[test]
    fn wire_conversion_matches_messages_format() {
        let messages = vec![
            Message::text(Role::System, "Be helpful"),
            Message::text(Role::System, "Be brief"),
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
                        signature: Some("sig".into()),
                    },
                    // Unsigned reasoning can't be verified, so it is dropped.
                    ContentBlock::Thinking {
                        text: "unsigned".into(),
                        signature: None,
                    },
                    ContentBlock::ToolUse {
                        id: "toolu_1".into(),
                        name: "add".into(),
                        input: serde_json::json!({"a": 1}),
                    },
                    ContentBlock::ToolUse {
                        id: "toolu_2".into(),
                        name: "now".into(),
                        input: serde_json::Value::Null,
                    },
                ],
            },
            Message {
                role: Role::User,
                content: vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "toolu_1".into(),
                        content: "2".into(),
                        is_error: false,
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "toolu_2".into(),
                        content: "no clock".into(),
                        is_error: true,
                    },
                ],
            },
            // A later system message becomes user text, merged into the
            // preceding user turn.
            Message::text(Role::System, "Wrap up now"),
        ];

        let (system, wire) = to_wire_messages(&messages);
        assert_eq!(system.as_deref(), Some("Be helpful\n\nBe brief"));
        assert_eq!(
            serde_json::to_value(wire).unwrap(),
            serde_json::json!([
                {"role": "user", "content": [
                    {"type": "text", "text": "What is in this image?"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": "AAAA"}}
                ]},
                {"role": "assistant", "content": [
                    {"type": "thinking", "thinking": "I should add these.", "signature": "sig"},
                    {"type": "tool_use", "id": "toolu_1", "name": "add", "input": {"a": 1}},
                    {"type": "tool_use", "id": "toolu_2", "name": "now", "input": {}}
                ]},
                {"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": "toolu_1", "content": "2"},
                    {"type": "tool_result", "tool_use_id": "toolu_2", "content": "no clock", "is_error": true},
                    {"type": "text", "text": "Wrap up now"}
                ]}
            ])
        );
    }

    #[test]
    fn response_keeps_signed_thinking_and_skips_unknown_blocks() {
        let response: MessagesResponse = serde_json::from_value(serde_json::json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-5",
            "content": [
                {"type": "thinking", "thinking": "", "signature": "sig"},
                {"type": "redacted_thinking", "data": "opaque"},
                {"type": "text", "text": "Adding."},
                {"type": "tool_use", "id": "toolu_1", "name": "add", "input": {"a": 1}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 10, "output_tokens": 5,
                      "cache_creation_input_tokens": 0, "cache_read_input_tokens": 90}
        }))
        .unwrap();

        let response = to_response(response);
        assert_eq!(response.finish_reason.as_deref(), Some("tool_use"));
        assert_eq!(response.text(), "Adding.");
        assert!(matches!(
            &response.content[0],
            ContentBlock::Thinking { text, signature: Some(s) } if text.is_empty() && s == "sig"
        ));
        assert_eq!(response.content.len(), 3);
        let usage = response.usage.unwrap();
        assert_eq!((usage.prompt_tokens, usage.total_tokens), (100, 105));
    }

    #[test]
    fn stream_events_reassemble_into_ordered_blocks() {
        let events = [
            r#"{"type":"message_start","message":{"id":"msg_1","model":"claude-opus-5"}}"#,
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"thinking","thinking":"","signature":""}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Need a sum."}}"#,
            r#"{"type":"content_block_delta","index":0,"delta":{"type":"signature_delta","signature":"sig"}}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"content_block_start","index":1,"content_block":{"type":"text","text":""}}"#,
            r#"{"type":"content_block_delta","index":1,"delta":{"type":"text_delta","text":"Adding."}}"#,
            r#"{"type":"content_block_stop","index":1}"#,
            r#"{"type":"ping"}"#,
            r#"{"type":"content_block_start","index":2,"content_block":{"type":"tool_use","id":"toolu_1","name":"add","input":{}}}"#,
            r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"{\"a\":"}}"#,
            r#"{"type":"content_block_delta","index":2,"delta":{"type":"input_json_delta","partial_json":"1}"}}"#,
            r#"{"type":"content_block_stop","index":2}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":20}}"#,
            r#"{"type":"message_stop"}"#,
        ];

        let mut parser = StreamParser::default();
        let mut acc = BlockAccumulator::default();
        for event in events {
            if let Some(delta) = parser.parse(event).unwrap() {
                // Tool call indices are dense even though the tool use was
                // content block 2.
                for tc in &delta.tool_calls {
                    assert_eq!(tc.index, 0);
                }
                acc.push(delta);
            }
        }
        let (blocks, finish) = acc.finish().unwrap();
        assert_eq!(finish.as_deref(), Some("tool_use"));
        assert!(matches!(
            &blocks[0],
            ContentBlock::Thinking { text, signature: Some(s) } if text == "Need a sum." && s == "sig"
        ));
        assert!(matches!(&blocks[1], ContentBlock::Text { text } if text == "Adding."));
        assert!(matches!(
            &blocks[2],
            ContentBlock::ToolUse { id, input, .. } if id == "toolu_1" && input == &serde_json::json!({"a": 1})
        ));
    }

    #[test]
    fn stream_error_event_is_an_error() {
        let err = StreamParser::default()
            .parse(r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#)
            .unwrap_err();
        assert!(err.to_string().contains("overloaded_error"));
    }
}
