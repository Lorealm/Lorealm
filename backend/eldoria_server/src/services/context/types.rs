use crate::services::context::{ContextNode, NodeContent, NodePath};

pub struct CreateNode {
    pub title: String,
    pub description: String,
    pub content: NodeContent,
    /// Path to parent group (None = root level)
    pub parent_path: Option<NodePath>,
    /// Index within parent (None = append)
    pub index: Option<usize>,
}

pub struct NodeWithPath {
    pub node: ContextNode,
    pub path: NodePath,
}

pub struct UpdateNode {
    pub title: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub content: Option<NodeContent>,
}

pub struct MoveNode {
    pub new_parent_path: Option<NodePath>,
    pub new_index: usize
}

pub struct MovedNode {
    pub new_path: NodePath
}