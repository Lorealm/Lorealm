use eldoria_storage::models::ConversationMessage;

/// How `edit_message` treats the original.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    /// Mutate content in place. The continuation below the message is kept and
    /// stays on the active path.
    InPlace,
    /// Leave the original untouched; create a new sibling with the new content
    /// and make it the active leaf. The original (and everything below it) stays
    /// in the tree, reachable via `switch_to`.
    Branch,
}

/// Sibling set around a message, for rendering a "‹ 2 / 3 ›" branch switcher.
#[derive(Debug, Clone)]
pub struct BranchInfo {
    /// Siblings ordered by `order`.
    pub siblings: Vec<ConversationMessage>,
    /// Index into `siblings` that the active path runs through, if it passes
    /// through this sibling group at all.
    pub active_index: Option<usize>,
}

impl BranchInfo {
    pub fn total(&self) -> usize {
        self.siblings.len()
    }
}
