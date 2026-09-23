wit_bindgen::generate!({
    path: "../../../wit",
    world: "plugin",
});

use lorealm::plugin::host::{active_persona, register_hook};
use lorealm::plugin::types::{Hook, Message, Role};

struct Hello;

impl Guest for Hello {
    fn init() -> Result<(), String> {
        register_hook(Hook::PromptBuild)
    }

    fn handle(event: Event) -> Result<Event, String> {
        match event {
            Event::PromptBuild(mut prompt) => {
                // Lets the host's call timeout be tested.
                if prompt.last().is_some_and(|m| m.content == "loop forever") {
                    loop {}
                }
                let persona = active_persona()?;
                prompt.push(Message {
                    role: Role::System,
                    content: format!("Hello, {}!", persona.name),
                });
                Ok(Event::PromptBuild(prompt))
            }
        }
    }
}

export!(Hello);
