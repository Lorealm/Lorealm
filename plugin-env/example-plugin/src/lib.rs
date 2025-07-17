use plugin_api::bindings::eldoria_hub::api::log;
use plugin_api::bindings::{self, export};
use plugin_api::bindings::exports::eldoria_hub::api;
use plugin_api::Plugin;

struct MyPlugin {}

impl Plugin for MyPlugin {
    fn init() -> () {
        log::log(&"Called from plugin setup!", log::LogLevel::Info);
    }
    
    fn component_registered() -> () {
        todo!()
    }
}

impl api::load_events::Guest for MyPlugin {
    fn on_permissions_granted() -> () {
        log::log(&"Permissions granted! Proceeding to register component...", log::LogLevel::Info);
        bindings::register_component("component.js");
    }

    fn on_plugin_loaded() -> () {
        log::log(&"Plugin loaded!", log::LogLevel::Info);
    }
}

impl api::prompt_processing::Guest for MyPlugin {
    fn process_prompt(prompt: String) -> () {
        todo!()
    }
}

export!(MyPlugin with_types_in plugin_api::bindings);