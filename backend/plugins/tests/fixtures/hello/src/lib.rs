wit_bindgen::generate!({
    path: "../../../wit",
    world: "plugin",
});

use lorealm::plugin::host::{active_persona, delete_data, load_data, persist_data, register_hook};
use lorealm::plugin::types::{Hook, Message, Role};

struct Hello;

impl Guest for Hello {
    fn init() -> Result<(), String> {
        register_hook(Hook::PromptBuild)
    }

    fn handle(event: Event) -> Result<Event, String> {
        match event {
            Event::PromptBuild(mut prompt) => {
                // Test controls: the last message's content.
                match prompt.last().map(|m| m.content.as_str()) {
                    Some("loop forever") => loop {},
                    Some("forget") => delete_data("greetings")?,
                    _ => {}
                }
                let persona = active_persona()?;
                let count = greetings()? + 1;
                persist_data("greetings", count.to_string().as_bytes())?;
                prompt.push(Message {
                    role: Role::System,
                    content: format!("Hello, {}! #{count}", persona.name),
                });
                Ok(Event::PromptBuild(prompt))
            }
        }
    }
}

/// How many times this user has been greeted.
fn greetings() -> Result<u32, String> {
    let Some(bytes) = load_data("greetings")? else {
        return Ok(0);
    };
    String::from_utf8(bytes)
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| "corrupt greeting count".to_owned())
}

export!(Hello);
