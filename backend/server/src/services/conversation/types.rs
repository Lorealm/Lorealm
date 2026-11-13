#[cfg(test)] // temporary until needed
use lorealm_storage::models::ConversationMessage;

/// How `edit_message` treats the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Mutate content in place.
    InPlace,
    /// Leave the original untouched; create a new sibling with the new content
    /// and make it the active leaf..
    #[allow(dead_code)]
    Branch,
}

/// Sibling set around a message, for rendering a "‹ 2 / 3 ›" branch switcher.
#[derive(Debug, Clone)]
#[cfg(test)] // temporary until needed
pub struct BranchInfo {
    /// Siblings ordered by `order`.
    pub siblings: Vec<ConversationMessage>,
    /// Index into `siblings` that the active path runs through
    pub active_index: Option<usize>,
}

#[cfg(test)] // temporary until needed
impl BranchInfo {
    pub fn total(&self) -> usize {
        self.siblings.len()
    }
}
