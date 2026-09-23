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
