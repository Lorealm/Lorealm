use crate::{settings::context_settings::ContextSettings, systems::context_template::{ContextError, ContextTemplate, ContextType, TemplateEntry, TemplateGroup}};

#[derive(Clone)]
pub struct ContextBuilder {
    settings: Option<ContextSettings>,
    template: Option<ContextTemplate>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self { settings: None, template: None }
    }

    pub fn add_entry(&mut self, entry: TemplateEntry, at: ContextCoordinate) -> Result<(), ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        let template = self.template.as_mut().unwrap();
        if let Some(inner_id) = at.inner_id {
            // outer_id = group index
            if let Some(group) = template.groups.get_mut(at.outer_id) {
                if inner_id <= group.entries.len() {
                    group.entries.insert(inner_id, entry);
                    template.sort();
                    template.reassign_orders();
                } else {
                    return Err(ContextError::OutOfBounds);
                }
                Ok(())
            } else {
                Err(ContextError::OutOfBounds)
            }
        } else {
            // outer_id = root entry index
            if at.outer_id <= template.entries.len() {
                template.entries.insert(at.outer_id, entry);
                template.sort();
                template.reassign_orders();
                Ok(())
            } else {
                Err(ContextError::OutOfBounds)
            }
        }
    }
    
    pub fn add_group(&mut self, group: TemplateGroup, at: usize) -> Result<(), ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        let template = self.template.as_mut().unwrap();
        if at <= template.groups.len() {
            template.groups.insert(at, group);
            Ok(())
        } else {
            Err(ContextError::OutOfBounds)
        }
    }

    pub fn build(&self) -> Result<String, ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        if let None = self.settings {
            return Err(ContextError::NotInitialized);
        }

        let template = self.template.as_ref().unwrap();
        let settings = self.settings.as_ref().unwrap();

        match template.context_type {
            ContextType::ChatCompletion => Ok(build_context_chat_completion(settings, template)),
            ContextType::TextCompletion => Ok(build_context_text_completion(settings, template)),
        }
    }
}

pub struct ContextCoordinate {
    outer_id: usize,
    inner_id: Option<usize>
}

impl From<(usize, usize)> for ContextCoordinate {
    fn from(value: (usize, usize)) -> Self {
        ContextCoordinate { outer_id: value.0, inner_id: Some(value.1) }
    }
}

fn build_context_chat_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}

fn build_context_text_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}