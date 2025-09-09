use uuid::Uuid;

use crate::{api::state::context_template::{templates::ContextTemplateError, types::ContextPosition}, settings::context_settings::ContextSettings, state::AppState, systems::context_template::{ContextError, ContextTemplate, ContextType, TemplateEntry, TemplateGroup, TemplateItem}};

#[derive(Clone)]
/// A function that takes an context template and settings. 
/// It's used to shield the underlying data from the system, and has functions to build the context to the desired format
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

    pub async fn save(&self, app_state: &AppState, session_token: &str) -> Result<(), ContextTemplateError> {
        if self.template.is_none() {
            return Err(ContextTemplateError::ContextError(ContextError::NotInitialized));
        }
        let resource_manager = app_state.resource_manager.lock().await;
        let user_resources = resource_manager.user_resources(session_token, app_state).await;
        
        if let Ok(user_resources) = user_resources {
            user_resources.save_context_template(self.template.as_ref().unwrap().clone(), app_state).await
                .map_err(|e| ContextTemplateError::Resource(e))
        } else {
            let context_error = ContextError::ContentResolution("Could not retrieve user resource manager".into());
            Err(ContextTemplateError::ContextError(context_error))
        }
    }

    pub fn add_entry(&mut self, entry: TemplateEntry, at: ContextCoordinate) -> Result<(), ContextError> {
        if let None = self.template {
            return Err(ContextError::NotInitialized);
        }
        let template = self.template.as_mut().unwrap();
        if let Some(inner_id) = at.inner_id {
            // Add to group at outer_id
            let outer_index = if at.outer_id == usize::MAX {
                template.items.len() // Append to end
            } else {
                at.outer_id
            };
            if let Some(item) = template.items.get_mut(outer_index) {
                if let TemplateItem::Group(group) = item {
                    let inner_index = if inner_id == usize::MAX {
                        group.entries.len() // Append to end
                    } else {
                        inner_id
                    };
                    if inner_index <= group.entries.len() {
                        log::debug!("Adding new entry with coordinates: ({}, {})", outer_index, inner_index);
                        group.entries.insert(inner_index, entry);
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
            let outer_index = if at.outer_id == usize::MAX {
                template.items.len() // Append to end
            } else {
                at.outer_id
            };
            if outer_index <= template.items.len() {
                log::debug!("Adding new entry with outer id: ({})", outer_index);
                template.items.insert(outer_index, TemplateItem::Entry(entry));
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
        let insert_index = if at == usize::MAX {
            template.items.len() // Append to end
        } else {
            at
        };
        if insert_index <= template.items.len() {
            log::debug!("Adding new group at index: ({})", insert_index);
            template.items.insert(insert_index, TemplateItem::Group(group));
            template.reassign_orders();
            Ok(())
        } else {
            Err(ContextError::OutOfBounds)
        }
    }

    pub fn remove_entry(&mut self, id: Uuid) -> Result<(), ContextError> {
        if self.template.is_none() {
            return Err(ContextError::NotInitialized);
        }

        let template = self.template.as_mut().unwrap();

        for (outer_idx, item) in template.items.iter_mut().enumerate() {
            match item {
                TemplateItem::Entry(entry) if entry.id == id => {
                    log::debug!("Removing entry {} from root at index: {}", id, outer_idx);
                    template.items.remove(outer_idx);
                    template.reassign_orders();
                    return Ok(());
                }
                TemplateItem::Group(group) => {
                    if let Some(inner_idx) = group.entries.iter().position(|e| e.id == id) {
                        log::debug!("Removing entry {} from group at index: ({}, {})", id, outer_idx, inner_idx);
                        group.entries.remove(inner_idx);
                        template.reassign_orders();
                        return Ok(());
                    }
                }
                _ => {}
            }
        }

        Err(ContextError::NotFound)
    }

    pub fn remove_group(&mut self, id: Uuid) -> Result<(), ContextError> {
        if self.template.is_none() {
            return Err(ContextError::NotInitialized);
        }

        let template = self.template.as_mut().unwrap();

        if let Some(outer_idx) = template.items.iter().position(|item| {
            if let TemplateItem::Group(group) = item {
                group.id == id
            } else {
                false
            }
        }) {
            log::debug!("Removing group {} at index: {}", id, outer_idx);
            template.items.remove(outer_idx);
            template.reassign_orders();
            Ok(())
        } else {
            Err(ContextError::NotFound)
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

    pub fn move_group(&mut self, group_id: Uuid, to: ContextCoordinate) -> Result<(), ContextError> {
        if self.template.is_none() {
            return Err(ContextError::NotInitialized);
        }

        if to.inner_id.is_some() {
            return Err(ContextError::InvalidOperation("Cannot move group to inner coordinate".into()));
        }

        let template = self.template.as_mut().unwrap();

        // Find and remove the group from its current location
        let mut group_to_move = None;
        if let Some(outer_idx) = template.items.iter().position(|item| {
            if let TemplateItem::Group(group) = item {
                group.id == group_id
            } else {
                false
            }
        }) {
            group_to_move = Some(template.items.remove(outer_idx));
        }

        // Check if group was found
        let group_item = match group_to_move {
            Some(TemplateItem::Group(group)) => group,
            _ => return Err(ContextError::NotFound),
        };

        // Insert the group at the new location
        let insert_index = if to.outer_id == usize::MAX {
            template.items.len() // Append to end
        } else {
            to.outer_id
        };

        if insert_index <= template.items.len() {
            log::debug!("Moving group {} to root at ({})", group_id, insert_index);
            template.items.insert(insert_index, TemplateItem::Group(group_item));
            template.reassign_orders();
            Ok(())
        } else {
            Err(ContextError::OutOfBounds)
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

    pub fn get_entry_by_coordinate(&self, coordinate: &ContextCoordinate) -> Option<&TemplateEntry> {
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

    pub fn get_entry_by_coordinate_mut(&mut self, coordinate: &ContextCoordinate) -> Option<&mut TemplateEntry> {
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

    pub fn get_group_by_coordinate(&self, coordinate: &ContextCoordinate) -> Option<&TemplateGroup> {
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

    pub fn get_group_by_coordinate_mut(&mut self, coordinate: &ContextCoordinate) -> Option<&mut TemplateGroup> {
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

impl From<&ContextTemplate> for ContextBuilder {
    fn from(value: &ContextTemplate) -> Self {
        Self { settings: None, template: Some(value.clone()) }
    }
}

pub struct ContextCoordinate {
    pub outer_id: usize,
    pub inner_id: Option<usize>
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

impl TryFrom<(i32, Option<i32>)> for ContextCoordinate {
    type Error = ContextError;

    fn try_from(value: (i32, Option<i32>)) -> Result<Self, Self::Error> {
        // Helper function to convert i32 to usize with -1 as sentinel
        fn convert_index(val: i32) -> Result<usize, ContextError> {
            if val == -1 {
                // Use usize::MAX as the sentinel for "append"
                Ok(usize::MAX)
            } else if val < 0 {
                Err(ContextError::NegativeIndexNotAllowed(val))
            } else {
                // Check for overflow when casting to usize
                usize::try_from(val).map_err(|_| ContextError::Overflow(val))
            }
        }

        let outer_id = convert_index(value.0)?;
        let inner_id = value.1.map(convert_index).transpose()?;

        Ok(ContextCoordinate { outer_id, inner_id })
    }
}

// Convert Position to ContextCoordinate for use in the handler
impl TryFrom<ContextPosition> for ContextCoordinate {
    type Error = ContextError;

    fn try_from(value: ContextPosition) -> Result<Self, Self::Error> {
        value
            .get()
            .ok_or(ContextError::NotInitialized)?
            .try_into()
    }
}

fn build_context_chat_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}

fn build_context_text_completion(settings: &ContextSettings, template: &ContextTemplate) -> String {
    String::new()
}