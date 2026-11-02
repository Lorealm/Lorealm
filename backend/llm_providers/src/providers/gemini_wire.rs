//! The Gemini `generateContent` wire format, as served by Vertex AI. Google
//! AI Studio uses the same body, so it is kept apart from the provider.

use crate::{error::*, streaming::BlockAccumulator, types::*};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Convert domain messages to a `systemInstruction` plus `contents`.
///
/// Leading system messages become the system instruction; later ones are
/// sent as user text. Consecutive same-role turns are merged into one.
pub(super) fn to_wire_contents(messages: &[Message]) -> (Option<WireContent>, Vec<WireContent>) {
    let leading = messages
        .iter()
        .take_while(|m| matches!(m.role, Role::System))
        .count();
    let system_parts: Vec<WirePart> = messages[..leading]
        .iter()
        .flat_map(|m| &m.content)
        .filter_map(|block| match block {
            ContentBlock::Text { text } => Some(WirePart::new(PartData::Text(text.clone()))),
            _ => None,
        })
        .collect();
    let system = if system_parts.is_empty() {
        None
    } else {
        Some(WireContent {
            role: None,
            parts: system_parts,
        })
    };

    // A function response must name its function, but the domain result
    // only carries the call id, so remember which name each id belongs to.
    let mut call_names: HashMap<&str, &str> = HashMap::new();
    let mut contents: Vec<WireContent> = Vec::new();
    for msg in &messages[leading..] {
        let role = match msg.role {
            Role::Assistant => "model",
            Role::User | Role::System => "user",
        };
        let mut parts = Vec::new();
        // A signature belongs on the part that follows its thinking block.
        let mut pending_signature: Option<String> = None;
        for block in &msg.content {
            let data = match block {
                ContentBlock::Text { text } => PartData::Text(text.clone()),
                ContentBlock::Image { media_type, source } => match source {
                    ImageSource::Base64(data) => PartData::InlineData {
                        mime_type: media_type.clone(),
                        data: data.clone(),
                    },
                    ImageSource::Url(url) => PartData::FileData {
                        mime_type: media_type.clone(),
                        file_uri: url.clone(),
                    },
                },
                // Thought text is never sent back; only its signature is.
                ContentBlock::Thinking { signature, .. } => {
                    if signature.is_some() {
                        pending_signature = signature.clone();
                    }
                    continue;
                }
                ContentBlock::ToolUse { id, name, input } => {
                    call_names.insert(id, name);
                    PartData::FunctionCall {
                        name: name.clone(),
                        args: if input.is_null() {
                            serde_json::json!({})
                        } else {
                            input.clone()
                        },
                    }
                }
                ContentBlock::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } => PartData::FunctionResponse {
                    name: call_names
                        .get(tool_use_id.as_str())
                        .copied()
                        .unwrap_or(tool_use_id)
                        .to_string(),
                    response: function_response(content, *is_error),
                },
            };
            parts.push(WirePart {
                data,
                thought_signature: pending_signature.take(),
            });
        }
        if parts.is_empty() {
            continue;
        }
        match contents.last_mut() {
            Some(last) if last.role == Some(role) => last.parts.extend(parts),
            _ => contents.push(WireContent {
                role: Some(role),
                parts,
            }),
        }
    }
    (system, contents)
}

/// `functionResponse.response` must be a JSON object. Results that already
/// are one pass through; anything else is wrapped.
fn function_response(content: &str, is_error: bool) -> serde_json::Value {
    if is_error {
        return serde_json::json!({ "error": content });
    }
    match serde_json::from_str::<serde_json::Value>(content) {
        Ok(value @ serde_json::Value::Object(_)) => value,
        Ok(value) => serde_json::json!({ "result": value }),
        Err(_) => serde_json::json!({ "result": content }),
    }
}

/// Convert advertised tool definitions to the wire `tools` array.
pub(super) fn to_wire_tools(tools: Option<&Vec<ToolDef>>) -> Option<Vec<WireTool>> {
    // Callers may pass an empty list, but the API rejects a declaration group
    // with nothing in it, so the field is left out instead.
    tools.filter(|defs| !defs.is_empty()).map(|defs| {
        vec![WireTool {
            function_declarations: defs
                .iter()
                .map(|d| WireFunctionDeclaration {
                    name: d.name.clone(),
                    description: d.description.clone(),
                    // `parameters` only takes an OpenAPI subset (no type
                    // arrays, no `additionalProperties`); this field takes
                    // the JSON Schema that tools generate as-is.
                    parameters_json_schema: d.input_schema.clone(),
                })
                .collect(),
        }]
    })
}

#[derive(Serialize)]
pub(super) struct WireContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    parts: Vec<WirePart>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WirePart {
    #[serde(flatten)]
    data: PartData,
    #[serde(skip_serializing_if = "Option::is_none")]
    thought_signature: Option<String>,
}

impl WirePart {
    fn new(data: PartData) -> Self {
        Self {
            data,
            thought_signature: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
enum PartData {
    Text(String),
    InlineData {
        mime_type: String,
        data: String,
    },
    FileData {
        mime_type: String,
        file_uri: String,
    },
    FunctionCall {
        name: String,
        args: serde_json::Value,
    },
    FunctionResponse {
        name: String,
        response: serde_json::Value,
    },
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct WireTool {
    function_declarations: Vec<WireFunctionDeclaration>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WireFunctionDeclaration {
    name: String,
    description: String,
    parameters_json_schema: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    usage_metadata: Option<UsageMetadata>,
    #[serde(default)]
    model_version: String,
    prompt_feedback: Option<PromptFeedback>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<CandidateContent>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct CandidateContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResponsePart {
    text: Option<String>,
    /// Marks `text` as a thought summary rather than answer text.
    #[serde(default)]
    thought: bool,
    function_call: Option<ResponseFunctionCall>,
    thought_signature: Option<String>,
}

#[derive(Deserialize)]
struct ResponseFunctionCall {
    id: Option<String>,
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    thoughts_token_count: u32,
    #[serde(default)]
    total_token_count: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptFeedback {
    block_reason: Option<String>,
}

#[derive(Default)]
pub(super) struct DeltaParser {
    next_call: usize,
}

impl DeltaParser {
    /// One delta per part keeps each thought signature next to the part it
    /// was attached to (a `Delta` holds a single signature).
    pub(super) fn split_deltas(&mut self, response: GenerateResponse) -> Result<Vec<Delta>> {
        let Some(candidate) = response.candidates.into_iter().next() else {
            if let Some(reason) = response.prompt_feedback.and_then(|f| f.block_reason) {
                return Err(Error::Provider(format!("prompt blocked: {}", reason)));
            }
            // Usage-only chunks carry no candidate.
            return Ok(Vec::new());
        };

        let parts = candidate.content.map(|c| c.parts).unwrap_or_default();
        let mut deltas = Vec::with_capacity(parts.len() + 1);
        for part in parts {
            let mut delta = Delta {
                reasoning_signature: part.thought_signature,
                ..Delta::default()
            };
            match part.text {
                Some(text) if part.thought => delta.reasoning = text,
                Some(text) => delta.content = text,
                None => {}
            }
            if let Some(call) = part.function_call {
                let index = self.next_call;
                self.next_call += 1;
                delta.tool_calls.push(ToolCallDelta {
                    index,
                    id: Some(call.id.unwrap_or_else(|| format!("call_{}", index))),
                    name: Some(call.name),
                    arguments: if call.args.is_null() {
                        String::new()
                    } else {
                        call.args.to_string()
                    },
                });
            }
            deltas.push(delta);
        }
        if candidate.finish_reason.is_some() {
            deltas.push(Delta {
                finish_reason: candidate.finish_reason,
                ..Delta::default()
            });
        }
        Ok(deltas)
    }

    /// Parse one SSE `data:` payload.
    pub(super) fn parse(&mut self, data: &str) -> Result<Option<Vec<Delta>>> {
        Ok(Some(self.split_deltas(serde_json::from_str(data)?)?))
    }
}

/// Turn a complete (non-streamed) response into the domain response.
pub(super) fn to_response(mut response: GenerateResponse) -> Result<Response> {
    let model = std::mem::take(&mut response.model_version);
    let usage = response.usage_metadata.take().map(|u| Usage {
        prompt_tokens: u.prompt_token_count,
        // Thinking is billed as output but reported separately.
        completion_tokens: u.candidates_token_count + u.thoughts_token_count,
        total_tokens: u.total_token_count,
    });

    let mut acc = BlockAccumulator::default();
    for delta in DeltaParser::default().split_deltas(response)? {
        acc.push(delta);
    }
    let (content, finish_reason) = acc.finish()?;

    Ok(Response {
        content,
        model,
        usage,
        finish_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_conversion_matches_gemini_format() {
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
                        signature: Some("sig".into()),
                    },
                    ContentBlock::ToolUse {
                        id: "call_0".into(),
                        name: "add".into(),
                        input: serde_json::json!({"a": 1}),
                    },
                    ContentBlock::ToolUse {
                        id: "call_1".into(),
                        name: "now".into(),
                        input: serde_json::Value::Null,
                    },
                ],
            },
            Message {
                role: Role::User,
                content: vec![
                    ContentBlock::ToolResult {
                        tool_use_id: "call_0".into(),
                        content: "2".into(),
                        is_error: false,
                    },
                    ContentBlock::ToolResult {
                        tool_use_id: "call_1".into(),
                        content: "no clock".into(),
                        is_error: true,
                    },
                ],
            },
        ];

        let (system, contents) = to_wire_contents(&messages);
        assert_eq!(
            serde_json::to_value(system).unwrap(),
            serde_json::json!({"parts": [{"text": "Be helpful"}]})
        );
        assert_eq!(
            serde_json::to_value(contents).unwrap(),
            serde_json::json!([
                {"role": "user", "parts": [
                    {"text": "What is in this image?"},
                    {"inlineData": {"mimeType": "image/png", "data": "AAAA"}}
                ]},
                {"role": "model", "parts": [
                    {"functionCall": {"name": "add", "args": {"a": 1}}, "thoughtSignature": "sig"},
                    {"functionCall": {"name": "now", "args": {}}}
                ]},
                {"role": "user", "parts": [
                    {"functionResponse": {"name": "add", "response": {"result": 2}}},
                    {"functionResponse": {"name": "now", "response": {"error": "no clock"}}}
                ]}
            ])
        );
    }

    #[test]
    fn response_round_trips_signatures_onto_the_right_part() {
        let response: GenerateResponse = serde_json::from_value(serde_json::json!({
            "candidates": [{
                "content": {"role": "model", "parts": [
                    {"text": "Summing.", "thought": true},
                    {"functionCall": {"name": "add", "args": {"a": 1}}, "thoughtSignature": "sig"},
                    {"functionCall": {"name": "add", "args": {"a": 2}}}
                ]},
                "finishReason": "STOP"
            }],
            "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 5,
                              "thoughtsTokenCount": 7, "totalTokenCount": 22},
            "modelVersion": "gemini-3-pro"
        }))
        .unwrap();

        let response = to_response(response).unwrap();
        assert_eq!(response.model, "gemini-3-pro");
        assert_eq!(response.finish_reason.as_deref(), Some("STOP"));
        assert_eq!(response.usage.unwrap().completion_tokens, 12);
        assert!(matches!(
            &response.content[0],
            ContentBlock::Thinking { text, signature: Some(s) } if text == "Summing." && s == "sig"
        ));
        assert!(matches!(
            &response.content[1],
            ContentBlock::ToolUse { id, .. } if id == "call_0"
        ));
        assert!(matches!(
            &response.content[2],
            ContentBlock::ToolUse { id, .. } if id == "call_1"
        ));

        // Replaying the turn puts the signature back on the first call only.
        let (_, contents) = to_wire_contents(&[Message {
            role: Role::Assistant,
            content: response.content,
        }]);
        let parts = &serde_json::to_value(contents).unwrap()[0]["parts"];
        assert_eq!(parts[0]["thoughtSignature"], "sig");
        assert!(parts[1].get("thoughtSignature").is_none());
    }

    #[test]
    fn an_empty_tool_list_sends_no_tools() {
        // A declaration group with nothing in it is a 400.
        assert!(to_wire_tools(Some(&Vec::new())).is_none());
        assert!(to_wire_tools(None).is_none());
    }

    #[test]
    fn blocked_prompt_is_an_error() {
        let response: GenerateResponse = serde_json::from_value(serde_json::json!({
            "promptFeedback": {"blockReason": "SAFETY"}
        }))
        .unwrap();
        let err = to_response(response).unwrap_err();
        assert!(err.to_string().contains("SAFETY"));
    }

    #[test]
    fn tool_call_indices_continue_across_stream_chunks() {
        let mut parser = DeltaParser::default();
        let chunk = r#"{"candidates":[{"content":{"role":"model","parts":[{"functionCall":{"name":"add","args":{}}}]}}]}"#;
        let first = parser.parse(chunk).unwrap().unwrap();
        let second = parser.parse(chunk).unwrap().unwrap();
        assert_eq!(first[0].tool_calls[0].index, 0);
        assert_eq!(second[0].tool_calls[0].index, 1);
        assert_eq!(second[0].tool_calls[0].id.as_deref(), Some("call_1"));
    }
}
