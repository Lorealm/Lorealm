use dyn_clone::DynClone;

/// Important distinction, as message roles are not available in Text Completion
#[derive(Clone)]
pub enum ContextType {
    ChatCompletion,
    TextCompletion
}

/// Root level template.
#[derive(Clone)]
pub struct ContextTemplate {
    pub context_type: ContextType,
    pub name: String,
    pub description: String,
    pub groups: Vec<TemplateGroup>,
    pub entries: Vec<TemplateEntry>
}

impl ContextTemplate {
    /// Sorts groups and entries (recursively) by their `order` field.
    pub fn sort(&mut self) {
        // Sort root entries
        self.entries.sort_by_key(|e| e.order);

        // Sort groups
        self.groups.sort_by_key(|g| g.order);

        // Sort entries inside each group
        for group in &mut self.groups {
            group.entries.sort_by_key(|e| e.order);
        }
    }

    pub fn reassign_orders(&mut self) {
        // Root entries
        for (i, entry) in self.entries.iter_mut().enumerate() {
            entry.order = i;
        }

        // Groups
        for (i, group) in self.groups.iter_mut().enumerate() {
            group.order = i;

            // Entries inside each group
            for (j, entry) in group.entries.iter_mut().enumerate() {
                entry.order = j;
            }
        }
    }
}

/// A group of template entries. Can be nested or used directly in the root
#[derive(Clone)]
pub struct TemplateGroup {
    pub name: String,
    /// The order of the group, relative to it's root
    pub order: usize,
    pub entries: Vec<TemplateEntry>,
    pub enabled: bool
}

/// A template entry. Can be within a group or directly in the root
#[derive(Clone)]
pub struct TemplateEntry {
    pub name: String,
    pub content: Box<dyn ContextSource>,
    /// The order of the entry, relative to it's root
    pub order: usize,
    pub enabled: bool
}

#[derive(Debug)]
pub enum ContextError {
    ContentResolution(String),
    OutOfBounds,
    NotInitialized
}

#[async_trait::async_trait]
pub trait ContextSource: DynClone + Send + Sync {
    /// The content of the given source
    async fn content(&self) -> Result<String, ContextError>;
    fn source(&self) -> String;
    fn substitute_variables(&self) -> Result<String, ContextError> {
        todo!()
    }
}

#[derive(Clone)]
pub struct PlainTextSource {
    text: String
}

#[async_trait::async_trait]
impl ContextSource for PlainTextSource {
    async fn content(&self) -> Result<String, ContextError> {
        Ok(self.text.clone())
    }
    fn source(&self) -> String {
        String::from("eldoria.plain_text")
    }
}

dyn_clone::clone_trait_object!(ContextSource);