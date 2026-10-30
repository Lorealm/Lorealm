wit_bindgen::generate!({
    path: "../../backend/plugins/wit",
    world: "plugin",
});

use std::collections::HashMap;
use std::sync::Mutex;

use lorealm::plugin::host::{generate, log, messages, register_hook, send_frontend, settings};
use lorealm::plugin::types::{FrontendCall, Generated, Hook, Level, Message, MessageRange, Role};
use serde_json::{Value, json};

/// How much of the story HypeBot reads, from the end, in characters.
const READ: usize = 6000;

/// The last comment in each conversation, so it's still there when the chat
/// is opened again.
static LAST: Mutex<Option<HashMap<i64, String>>> = Mutex::new(None);

struct HypeBot;

impl Guest for HypeBot {
    fn init() -> Result<(), String> {
        register_hook(Hook::AfterGenerate)
    }

    fn handle(event: Event) -> Result<Response, String> {
        match event {
            Event::AfterGenerate(Generated { conversation, .. }) => {
                let settings: Value =
                    serde_json::from_str(&settings()?).map_err(|e| e.to_string())?;
                if settings["auto"] == true {
                    send_frontend(
                        "thinking",
                        &json!({ "conversation": conversation.to_string() }).to_string(),
                    )?;
                    let payload = match comment(conversation) {
                        Ok(text) => {
                            json!({ "conversation": conversation.to_string(), "text": text })
                        }
                        Err(error) => {
                            log(Level::Warn, &error);
                            json!({ "conversation": conversation.to_string(), "error": error })
                        }
                    };
                    send_frontend("comment", &payload.to_string())?;
                }
                Ok(Response::AfterGenerate)
            }
            Event::Frontend(FrontendCall {
                name, conversation, ..
            }) => {
                let Some(conversation) = conversation else {
                    return Err("Open a chat first.".into());
                };
                let value = match name.as_str() {
                    "last" => json!(last(conversation)),
                    "comment" => json!(comment(conversation)?),
                    other => return Err(format!("unknown call `{other}`")),
                };
                Ok(Response::Frontend(value.to_string()))
            }
            _ => Err("unexpected event".into()),
        }
    }
}

fn last(conversation: i64) -> Option<String> {
    LAST.lock().unwrap().as_ref()?.get(&conversation).cloned()
}

/// A fresh comment on the end of `conversation`, remembered as its last.
fn comment(conversation: i64) -> Result<String, String> {
    let settings: Value = serde_json::from_str(&settings()?).map_err(|e| e.to_string())?;
    // `null` asks the user's active profile.
    let profile = settings["profile"].as_str().and_then(|id| id.parse().ok());
    let story = transcript(&messages(conversation, MessageRange::Latest(8))?);
    if story.is_empty() {
        return Err("There's nothing to read yet.".into());
    }
    let reply = generate(
        &[
            Message {
                role: Role::System,
                content: settings["prompt"].as_str().unwrap_or_default().into(),
            },
            Message {
                role: Role::User,
                content: story,
            },
        ],
        profile,
    )?;
    let text = reply.trim().trim_matches('"').trim().to_owned();
    if text.is_empty() {
        return Err("HypeBot is speechless.".into());
    }
    LAST.lock()
        .unwrap()
        .get_or_insert_default()
        .insert(conversation, text.clone());
    Ok(text)
}

/// The story so far as plain text, cut to its last `READ` characters.
fn transcript(messages: &[lorealm::plugin::types::ChatMessage]) -> String {
    let story = messages
        .iter()
        .filter(|m| !matches!(m.role, Role::System))
        .map(|m| m.content.trim())
        .filter(|c| !c.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let skip = story.chars().count().saturating_sub(READ);
    story.chars().skip(skip).collect()
}

export!(HypeBot);
