use uuid::Uuid;

use crate::{resources::resource_manager::ResourceManager, settings::context_settings::ContextSettings, state::AppState, systems::context_template::{ContextError, ContextTemplate, ContextType, TemplateEntry, TemplateGroup, TemplateItem}};

#[derive(Clone)]
pub struct ContextBuilder {
    settings: Option<ContextSettings>,
    template: Option<ContextTemplate>,
}

impl ContextBuilder {
    pub fn new() -> Self {
        Self { settings: None, template: None }
    }

    pub fn set_template(&mut self, template: ContextTemplate) {
        self.template = Some(template)
    }

    pub fn get_template(&self) -> Option<&ContextTemplate> {
        self.template.as_ref()
    }

    pub async fn save(&self, app_state: &AppState, session_token: String) -> Result<(), String> {
        if self.template.is_none() {
            return Err("Resource Template not defined".into());
        }
        let resource_manager = app_state.resource_manager.lock().await;
        let user_resources = resource_manager.user_resources(session_token, app_state).await;
        
        if let Ok(user_resources) = user_resources {
            user_resources.save_context_template(self.template.as_ref().unwrap().clone(), app_state).await
                .map_err(|e| e.to_string())
        } else {
            Err("Could not retrieve user resource manager".into())
        }
    }

    pub fn add_entry(&mut self, entry: TemplateEntry, at: ContextCoordinate) -> Result<(), ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        let template = self.template.as_mut().unwrap();
        if let Some(inner_id) = at.inner_id {
            // Add to group at outer_id
            if let Some(item) = template.items.get_mut(at.outer_id) {
                if let TemplateItem::Group(group) = item {
                    if inner_id <= group.entries.len() {
                        log::debug!("Adding new entry with inner id: ({}, {})", at.outer_id, inner_id);
                        group.entries.insert(inner_id, entry);
                        template.reassign_orders();
                        Ok(())
                    } else {
                        return Err(ContextError::OutOfBounds);
                    }
                } else {
                    return Err(ContextError::OutOfBounds);
                }
            } else {
                return Err(ContextError::OutOfBounds);
            }
        } else {
            // Add root entry
            if at.outer_id <= template.items.len() {
                log::debug!("Adding new entry with outer id: ({})", at.outer_id);
                template.items.insert(at.outer_id, TemplateItem::Entry(entry));
                template.reassign_orders();
                Ok(())
            } else {
                return Err(ContextError::OutOfBounds);
            }
        }
    }
    
    pub fn add_group(&mut self, group: TemplateGroup, at: usize) -> Result<(), ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        let template = self.template.as_mut().unwrap();
        if at <= template.items.len() {
            template.items.insert(at, TemplateItem::Group(group));
            template.reassign_orders();
            Ok(())
        } else {
            Err(ContextError::OutOfBounds)
        }
    }

    pub fn move_entry(&mut self, entry_id: Uuid, to: ContextCoordinate) -> Result<(), ContextError> {
        if self.template.is_none() {
            return Err(ContextError::NotInitialized);
        }

        let template = self.template.as_mut().unwrap();

        // Find and remove the entry from its current location
        let mut entry_to_move = None;

        // Search through root items and groups to find the entry
        for (outer_idx, item) in template.items.iter_mut().enumerate() {
            match item {
                TemplateItem::Entry(entry) if entry.id == entry_id => {
                    entry_to_move = Some(template.items.remove(outer_idx));
                    break;
                }
                TemplateItem::Group(group) => {
                    if let Some(inner_idx) = group.entries.iter().position(|e| e.id == entry_id) {
                        entry_to_move = Some(TemplateItem::Entry(group.entries.remove(inner_idx)));
                        break;
                    }
                }
                _ => {}
            }
        }

        // Check if entry was found
        let entry_item = match entry_to_move {
            Some(TemplateItem::Entry(entry)) => entry,
            _ => return Err(ContextError::NotFound),
        };

        // Insert the entry at the new location
        if let Some(inner_id) = to.inner_id {
            // Moving to a group
            if let Some(item) = template.items.get_mut(to.outer_id) {
                if let TemplateItem::Group(group) = item {
                    if inner_id <= group.entries.len() {
                        log::debug!(
                            "Moving entry {} to group at ({}, {})",
                            entry_id,
                            to.outer_id,
                            inner_id
                        );
                        group.entries.insert(inner_id, entry_item);
                        template.reassign_orders();
                        return Ok(());
                    }
                }
                Err(ContextError::OutOfBounds)
            } else {
                Err(ContextError::OutOfBounds)
            }
        } else {
            // Moving to root level
            if to.outer_id <= template.items.len() {
                log::debug!("Moving entry {} to root at ({})", entry_id, to.outer_id);
                template.items.insert(to.outer_id, TemplateItem::Entry(entry_item));
                template.reassign_orders();
                Ok(())
            } else {
                Err(ContextError::OutOfBounds)
            }
        }
    }

    pub fn get_entry(&self, id: Uuid) -> Option<&TemplateEntry> {
        if let Some(template) = &self.template {
            for item in &template.items {
                match item {
                    TemplateItem::Entry(e) if e.id == id => return Some(e),
                    TemplateItem::Group(g) => {
                        if let Some(e) = g.entries.iter().find(|e| e.id == id) {
                            return Some(e);
                        }
                    },
                    &TemplateItem::Entry(_) => return None
                }
            }
        }
        None
    }

    pub fn get_entry_mut(&mut self, id: Uuid) -> Option<&mut TemplateEntry> {
        if let Some(template) = self.template.as_mut() {
            for item in template.items.iter_mut() {
                match item {
                    TemplateItem::Entry(e) if e.id == id => return Some(e),
                    TemplateItem::Group(g) => {
                        if let Some(e) = g.entries.iter_mut().find(|e| e.id == id) {
                            return Some(e);
                        }
                    },
                    TemplateItem::Entry(_) => return None
                }
            }
        }
        None
    }

    pub fn get_entry_by_coordinate(&self, coordinate: ContextCoordinate) -> Option<&TemplateEntry> {
        if let Some(template) = &self.template {
            if let Some(inner_id) = coordinate.inner_id {
                // Get entry from group
                if let Some(item) = template.items.get(coordinate.outer_id) {
                    if let TemplateItem::Group(group) = item {
                        return group.entries.get(inner_id);
                    }
                }
            } else {
                // Get root entry
                if let Some(item) = template.items.get(coordinate.outer_id) {
                    if let TemplateItem::Entry(entry) = item {
                        return Some(entry);
                    }
                }
            }
        }
        None
    }

    pub fn get_entry_by_coordinate_mut(&mut self, coordinate: ContextCoordinate) -> Option<&mut TemplateEntry> {
        if let Some(template) = self.template.as_mut() {
            if let Some(inner_id) = coordinate.inner_id {
                // Get entry from group
                if let Some(item) = template.items.get_mut(coordinate.outer_id) {
                    if let TemplateItem::Group(group) = item {
                        return group.entries.get_mut(inner_id);
                    }
                }
            } else {
                // Get root entry
                if let Some(item) = template.items.get_mut(coordinate.outer_id) {
                    if let TemplateItem::Entry(entry) = item {
                        return Some(entry);
                    }
                }
            }
        }
        None
    }

    pub fn get_group(&self, id: Uuid) -> Option<&TemplateGroup> {
        if let Some(template) = &self.template {
            for item in &template.items {
                if let TemplateItem::Group(g) = item {
                    if g.id == id {
                        return Some(g);
                    }
                }
            }
        }
        None
    }

    pub fn get_group_mut(&mut self, id: Uuid) -> Option<&mut TemplateGroup> {
        if let Some(template) = self.template.as_mut() {
            for item in template.items.iter_mut() {
                if let TemplateItem::Group(g) = item {
                    if g.id == id {
                        return Some(g);
                    }
                }
            }
        }
        None
    }

    pub fn get_group_by_coordinate(&self, coordinate: ContextCoordinate) -> Option<&TemplateGroup> {
        if let Some(template) = &self.template {
            if coordinate.inner_id.is_none() {
                if let Some(item) = template.items.get(coordinate.outer_id) {
                    if let TemplateItem::Group(group) = item {
                        return Some(group);
                    }
                }
            }
        }
        None
    }

    pub fn get_group_by_coordinate_mut(&mut self, coordinate: ContextCoordinate) -> Option<&mut TemplateGroup> {
        if let Some(template) = self.template.as_mut() {
            if coordinate.inner_id.is_none() {
                if let Some(item) = template.items.get_mut(coordinate.outer_id) {
                    if let TemplateItem::Group(group) = item {
                        return Some(group);
                    }
                }
            }
        }
        None
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

impl From<(usize, Option<usize>)> for ContextCoordinate {
    fn from(value: (usize, Option<usize>)) -> Self {
        ContextCoordinate { outer_id: value.0, inner_id: value.1 }
    }
}

fn build_context_chat_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}

fn build_context_text_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}