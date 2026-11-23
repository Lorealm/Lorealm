//! Text completion for OpenAI-compatible servers

use crate::{error::*, types::*};
use serde::{Deserialize, Serialize};

/// The text wrapped around each message so a model sees the turn format it
/// was trained on, e.g. ChatML's `<|im_start|>user\n` … `<|im_end|>\n`.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[serde(deny_unknown_fields, default)]
pub struct InstructTemplate {
    pub system_prefix: String,
    pub system_suffix: String,
    pub user_prefix: String,
    pub user_suffix: String,
    pub assistant_prefix: String,
    pub assistant_suffix: String,
}

impl InstructTemplate {
    pub fn render(&self, messages: &[Message]) -> String {
        let mut prompt = String::new();
        for message in messages {
            let (prefix, suffix) = match message.role {
                Role::System => (&self.system_prefix, &self.system_suffix),
                Role::User => (&self.user_prefix, &self.user_suffix),
                Role::Assistant => (&self.assistant_prefix, &self.assistant_suffix),
            };
            prompt.push_str(prefix);
            for block in &message.content {
                if let ContentBlock::Text { text } = block {
                    prompt.push_str(text);
                }
            }
            prompt.push_str(suffix);
        }
        prompt.push_str(&self.assistant_prefix);
        prompt
    }

    fn stop_strings(&self) -> impl Iterator<Item = String> + '_ {
        [&self.assistant_suffix, &self.user_prefix]
            .into_iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    }
}

#[derive(Serialize)]
pub(super) struct CompletionRequest {
    model: String,
    prompt: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    min_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repetition_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

pub(super) fn to_request(
    model: &str,
    template: &InstructTemplate,
    messages: &[Message],
    params: &CommonParams,
    stream: bool,
) -> CompletionRequest {
    let stop: Vec<String> = params
        .stop
        .iter()
        .flatten()
        .cloned()
        .chain(template.stop_strings())
        .collect();

    CompletionRequest {
        model: model.to_string(),
        prompt: template.render(messages),
        temperature: params.temperature,
        max_tokens: params.max_tokens,
        top_p: params.top_p,
        frequency_penalty: params.frequency_penalty,
        presence_penalty: params.presence_penalty,
        seed: params.seed,
        top_k: params.top_k,
        min_p: params.min_p,
        repetition_penalty: params.repetition_penalty,
        stop: (!stop.is_empty()).then_some(stop),
        stream: stream.then_some(true),
    }
}

/// A `/completions` response; streamed chunks have the same shape.
#[derive(Deserialize)]
pub(super) struct CompletionResponse {
    choices: Vec<CompletionChoice>,
    usage: Option<WireUsage>,
    #[serde(default)]
    model: String,
}

#[derive(Deserialize)]
struct CompletionChoice {
    #[serde(default)]
    text: String,
    finish_reason: Option<String>,
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

pub(super) fn to_response(response: CompletionResponse) -> Result<Response> {
    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| Error::Provider("response contained no choices".to_string()))?;

    let content = match choice.text.is_empty() {
        true => vec![],
        false => vec![ContentBlock::Text { text: choice.text }],
    };

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

/// Parse one SSE `data:` payload; `Ok(None)` ends the stream, as in
/// `openai_wire::to_delta`.
pub(super) fn to_delta(data: &str) -> Result<Option<Delta>> {
    if data == "[DONE]" {
        return Ok(None);
    }
    let chunk: CompletionResponse = serde_json::from_str(data)?;
    let Some(choice) = chunk.choices.into_iter().next() else {
        return Ok(None);
    };
    Ok(Some(Delta {
        content: choice.text,
        finish_reason: choice.finish_reason,
        ..Delta::default()
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chatml() -> InstructTemplate {
        InstructTemplate {
            system_prefix: "<|im_start|>system\n".into(),
            system_suffix: "<|im_end|>\n".into(),
            user_prefix: "<|im_start|>user\n".into(),
            user_suffix: "<|im_end|>\n".into(),
            assistant_prefix: "<|im_start|>assistant\n".into(),
            assistant_suffix: "<|im_end|>\n".into(),
        }
    }

    #[test]
    fn renders_turns_and_opens_the_reply() {
        let messages = [
            Message::text(Role::System, "Narrate."),
            Message::text(Role::User, "I open the door."),
            Message::text(Role::Assistant, "It creaks."),
            Message::text(Role::User, "I step in."),
        ];

        assert_eq!(
            chatml().render(&messages),
            "<|im_start|>system\nNarrate.<|im_end|>\n\
             <|im_start|>user\nI open the door.<|im_end|>\n\
             <|im_start|>assistant\nIt creaks.<|im_end|>\n\
             <|im_start|>user\nI step in.<|im_end|>\n\
             <|im_start|>assistant\n"
        );
    }

    #[test]
    fn stops_at_the_turn_end_and_the_next_user_turn() {
        let params = CommonParams {
            stop: Some(vec!["THE END".into()]),
            ..CommonParams::default()
        };
        let request = to_request("m", &chatml(), &[], &params, false);

        assert_eq!(
            request.stop.unwrap(),
            ["THE END", "<|im_end|>", "<|im_start|>user"]
        );
    }

    #[test]
    fn parses_streamed_text() {
        let delta = to_delta(r#"{"choices":[{"text":"Hi","finish_reason":null}]}"#)
            .unwrap()
            .unwrap();
        assert_eq!(delta.content, "Hi");
        assert!(to_delta("[DONE]").unwrap().is_none());
    }
}
