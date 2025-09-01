use crate::{settings::context_settings::ContextSettings, systems::{context_template::{ContextTemplate, ContextType}, types::Message}};

pub struct ContextBuilder {
    settings: ContextSettings,
    template: ContextTemplate,
}

impl ContextBuilder {
    pub fn new(settings: ContextSettings, template: ContextTemplate) -> Self {
        Self { settings, template }
    }

    pub fn build(self) -> String {
        match self.template.context_type {
            ContextType::ChatCompletion => build_context_chat_completion(self.settings, self.template),
            ContextType::TextCompletion => build_context_text_completion(self.settings, self.template),
        }
    }
}

fn build_context_chat_completion(settings: ContextSettings, template: ContextTemplate) -> String {
    String::new()
}

fn build_context_text_completion(settings: ContextSettings, template: ContextTemplate) -> String {
    String::new()
}