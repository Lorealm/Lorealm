/// Important distinction, as message roles are not available in Text Completion
pub enum ContextType {
    ChatCompletion,
    TextCompletion
}

/// Root level template.
pub struct ContextTemplate {
    pub context_type: ContextType,
    pub name: String,
    pub description: String,
    pub groups: Vec<TemplateGroup>,
    pub entries: Vec<TemplateEntry>
}

/// A group of template entries. Can be nested or used directly in the root
pub struct TemplateGroup {
    pub name: String,
    /// The order of the group, relative to it's root
    pub order: i32,
    pub entries: Vec<TemplateEntry>,
    pub enabled: bool
}

/// A template entry. Can be within a group or directly in the root
pub struct TemplateEntry {
    pub name: String,
    pub content: String,
    /// The order of the entry, relative to it's root
    pub order: i32,
    pub enabled: bool
}

