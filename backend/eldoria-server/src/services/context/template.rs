use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

use super::{
    provider::SourceRegistry,
    types::{ContextNode, ContextTemplate, NodeContent, NodePath},
};

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Node not found: {0}")]
    NodeNotFound(Uuid),

    #[error("Duplicate source node: source '{source_id}' node '{node_id}' is already used")]
    DuplicateSourceNode { source_id: String, node_id: String },

    #[error("Invalid parent: {0} is not a group node")]
    InvalidParent(Uuid),

    #[error("Unknown source: {0}")]
    UnknownSource(String),

    #[error("Unknown source node: source '{source_id}' has no node '{node_id}'")]
    UnknownSourceNode { source_id: String, node_id: String },

    #[error("Invalid setting '{key}' for source '{source_id}' node '{node_id}'")]
    InvalidSetting {
        source_id: String,
        node_id: String,
        key: String,
    },

    #[error("Circular reference detected")]
    CircularReference,

    #[error("Cannot move node: would create circular reference")]
    WouldCreateCircle,

    #[error("Index out of bounds: {index} >= {len}")]
    IndexOutOfBounds { index: usize, len: usize },
}

impl ContextTemplate {
    /// Find a node by ID and return its path
    pub fn find_node_path(&self, node_id: Uuid) -> Option<NodePath> {
        fn search(nodes: &[ContextNode], target: Uuid, path: &mut NodePath) -> bool {
            for (i, node) in nodes.iter().enumerate() {
                path.push(i);
                if node.id == target {
                    return true;
                }
                if let NodeContent::Group { children, .. } = &node.content {
                    if search(children, target, path) {
                        return true;
                    }
                }
                path.pop();
            }
            false
        }

        let mut path = Vec::new();
        if search(&self.nodes, node_id, &mut path) {
            Some(path)
        } else {
            None
        }
    }

    /// Get a reference to a node by its ID
    pub fn get_node_by_id(&self, node_id: Uuid) -> Option<&ContextNode> {
        let path = self.find_node_path(node_id)?;
        self.get_node(&path)
    }

    /// Get a mutable reference to a node by its ID
    pub fn get_node_by_id_mut(&mut self, node_id: Uuid) -> Option<&mut ContextNode> {
        let path = self.find_node_path(node_id)?;
        self.get_node_mut(&path)
    }

    /// Get a reference to a node by path
    pub fn get_node(&self, path: &NodePath) -> Option<&ContextNode> {
        if path.is_empty() {
            return None;
        }

        let mut current_list = &self.nodes;
        let mut node = None;

        for (i, &index) in path.iter().enumerate() {
            node = current_list.get(index);
            match node {
                Some(n) => {
                    if i < path.len() - 1 {
                        if let NodeContent::Group { children, .. } = &n.content {
                            current_list = children;
                        } else {
                            return None;
                        }
                    }
                }
                None => return None,
            }
        }

        node
    }

    /// Get a mutable reference to a node by path
    pub fn get_node_mut(&mut self, path: &NodePath) -> Option<&mut ContextNode> {
        if path.is_empty() {
            return None;
        }

        let mut current_list = &mut self.nodes;

        for (i, &index) in path.iter().enumerate() {
            if i == path.len() - 1 {
                return current_list.get_mut(index);
            }

            let node = current_list.get_mut(index)?;
            if let NodeContent::Group { children, .. } = &mut node.content {
                current_list = children;
            } else {
                return None;
            }
        }

        None
    }

    /// Get the list containing a node at the given path
    fn get_parent_list(&self, path: &NodePath) -> Option<&Vec<ContextNode>> {
        if path.is_empty() {
            return None;
        }

        if path.len() == 1 {
            return Some(&self.nodes);
        }

        let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
        let parent = self.get_node(&parent_path)?;

        if let NodeContent::Group { children, .. } = &parent.content {
            Some(children)
        } else {
            None
        }
    }

    /// Get a mutable reference to the list containing a node
    fn get_parent_list_mut(&mut self, path: &NodePath) -> Option<&mut Vec<ContextNode>> {
        if path.is_empty() {
            return None;
        }

        if path.len() == 1 {
            return Some(&mut self.nodes);
        }

        let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
        let parent = self.get_node_mut(&parent_path)?;

        if let NodeContent::Group { children, .. } = &mut parent.content {
            Some(children)
        } else {
            None
        }
    }

    /// Add a new node to the template
    pub fn add_node(
        &mut self,
        node: ContextNode,
        parent_path: Option<&NodePath>,
        index: Option<usize>,
    ) -> Result<NodePath, TemplateError> {
        // Validate if it's a source node
        if let NodeContent::Source {
            source_id, node_id, ..
        } = &node.content
        {
            if self.is_source_node_used(source_id, node_id) {
                return Err(TemplateError::DuplicateSourceNode {
                    source_id: source_id.clone(),
                    node_id: node_id.clone(),
                });
            }
        }

        let target_list = match parent_path {
            None => &mut self.nodes,
            Some(path) => {
                let parent = self
                    .get_node_mut(path)
                    .ok_or_else(|| TemplateError::NodeNotFound(Uuid::nil()))?;

                if let NodeContent::Group { children, .. } = &mut parent.content {
                    children
                } else {
                    let parent_id = self.get_node(path).map(|n| n.id).unwrap_or(Uuid::nil());
                    return Err(TemplateError::InvalidParent(parent_id));
                }
            }
        };

        let insert_index = match index {
            Some(i) if i > target_list.len() => {
                return Err(TemplateError::IndexOutOfBounds {
                    index: i,
                    len: target_list.len(),
                });
            }
            Some(i) => i,
            None => target_list.len(),
        };

        target_list.insert(insert_index, node);

        let mut result_path = parent_path.cloned().unwrap_or_default();
        result_path.push(insert_index);

        self.mark_updated();
        Ok(result_path)
    }

    /// Remove a node by ID and return it
    pub fn remove_node(&mut self, node_id: Uuid) -> Result<ContextNode, TemplateError> {
        let path = self
            .find_node_path(node_id)
            .ok_or(TemplateError::NodeNotFound(node_id))?;

        let index = *path.last().unwrap();

        let removed = if path.len() == 1 {
            self.nodes.remove(index)
        } else {
            let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
            tracing::info!("Parent path: {:?}", parent_path);
            let parent = self
                .get_node_mut(&parent_path)
                .ok_or(TemplateError::NodeNotFound(node_id))?;

            if let NodeContent::Group { children, .. } = &mut parent.content {
                children.remove(index)
            } else {
                return Err(TemplateError::NodeNotFound(node_id));
            }
        };

        self.mark_updated();
        Ok(removed)
    }

    /// Move a node to a new position
    pub fn move_node(
        &mut self,
        node_id: Uuid,
        new_parent_path: Option<&NodePath>,
        new_index: usize,
    ) -> Result<NodePath, TemplateError> {
        // Resolve parent to UUID before we modify the tree
        let new_parent_id: Option<Uuid> = match new_parent_path {
            Some(path) => {
                let parent_node = self.get_node(path)
                    .ok_or_else(|| TemplateError::NodeNotFound(Uuid::nil()))?;

                // Check for circular reference
                if self.is_ancestor_of(node_id, parent_node.id) {
                    return Err(TemplateError::WouldCreateCircle);
                }

                // Verify it's a group
                if !matches!(parent_node.content, NodeContent::Group { .. }) {
                    return Err(TemplateError::InvalidParent(parent_node.id));
                }

                Some(parent_node.id)
            }
            None => None,
        };

        // Remove the node (this changes paths in the tree)
        let node = self.remove_node(node_id)?;

        // Find the parent's NEW path by ID (if we have a parent)
        let resolved_parent_path: Option<NodePath> = match new_parent_id {
            Some(parent_id) => {
                let path = self.find_node_path(parent_id)
                    .ok_or(TemplateError::NodeNotFound(parent_id))?;
                Some(path)
            }
            None => None,
        };

        // Now add the node using the resolved path
        self.add_node(node, resolved_parent_path.as_ref(), Some(new_index))
    }

    /// Check if `potential_ancestor` is an ancestor of `node_id`
    fn is_ancestor_of(&self, potential_ancestor: Uuid, node_id: Uuid) -> bool {
        fn check_children(nodes: &[ContextNode], ancestor_id: Uuid, target_id: Uuid) -> bool {
            for node in nodes {
                if node.id == ancestor_id {
                    // Found the potential ancestor, now check if target is a descendant
                    return contains_node(node, target_id);
                }
                if let NodeContent::Group { children, .. } = &node.content {
                    if check_children(children, ancestor_id, target_id) {
                        return true;
                    }
                }
            }
            false
        }

        fn contains_node(node: &ContextNode, target_id: Uuid) -> bool {
            if node.id == target_id {
                return true;
            }
            if let NodeContent::Group { children, .. } = &node.content {
                for child in children {
                    if contains_node(child, target_id) {
                        return true;
                    }
                }
            }
            false
        }

        check_children(&self.nodes, potential_ancestor, node_id)
    }

    /// Update a node's properties
    pub fn update_node(
        &mut self,
        node_id: Uuid,
        title: Option<String>,
        description: Option<String>,
        enabled: Option<bool>,
        content: Option<NodeContent>,
    ) -> Result<(), TemplateError> {
        // If updating to a source node, validate it's not already used
        if let Some(ref c) = content {
            if let NodeContent::Source {
                source_id,
                node_id: nid,
                ..
            } = c
            {
                // Check if this source node is used by a *different* node
                if self.is_source_node_used_by_other(source_id, nid, node_id) {
                    return Err(TemplateError::DuplicateSourceNode {
                        source_id: source_id.clone(),
                        node_id: nid.clone(),
                    });
                }
            }
        }

        let node = self
            .get_node_by_id_mut(node_id)
            .ok_or(TemplateError::NodeNotFound(node_id))?;

        if let Some(t) = title {
            node.title = t;
        }
        if let Some(d) = description {
            node.description = d;
        }
        if let Some(e) = enabled {
            node.enabled = e;
        }
        if let Some(c) = content {
            node.content = c;
        }

        self.mark_updated();
        Ok(())
    }

    /// Toggle a node's enabled state
    pub fn toggle_node(&mut self, node_id: Uuid) -> Result<bool, TemplateError> {
        let node = self
            .get_node_by_id_mut(node_id)
            .ok_or(TemplateError::NodeNotFound(node_id))?;

        node.enabled = !node.enabled;
        let new_state = node.enabled;
        self.mark_updated();
        Ok(new_state)
    }

    /// Check if a source node is already used in the template
    pub fn is_source_node_used(&self, source_id: &str, node_id: &str) -> bool {
        fn search(nodes: &[ContextNode], source_id: &str, node_id: &str) -> bool {
            for node in nodes {
                match &node.content {
                    NodeContent::Source {
                        source_id: sid,
                        node_id: nid,
                        ..
                    } => {
                        if sid == source_id && nid == node_id {
                            return true;
                        }
                    }
                    NodeContent::Group { children, .. } => {
                        if search(children, source_id, node_id) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        search(&self.nodes, source_id, node_id)
    }

    /// Check if a source node is used by a node other than the specified one
    fn is_source_node_used_by_other(
        &self,
        source_id: &str,
        node_id: &str,
        exclude_node_id: Uuid,
    ) -> bool {
        fn search(
            nodes: &[ContextNode],
            source_id: &str,
            node_id: &str,
            exclude_id: Uuid,
        ) -> bool {
            for node in nodes {
                if node.id == exclude_id {
                    continue;
                }
                match &node.content {
                    NodeContent::Source {
                        source_id: sid,
                        node_id: nid,
                        ..
                    } => {
                        if sid == source_id && nid == node_id {
                            return true;
                        }
                    }
                    NodeContent::Group { children, .. } => {
                        if search(children, source_id, node_id, exclude_id) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
            false
        }

        search(&self.nodes, source_id, node_id, exclude_node_id)
    }

    /// Get all used source nodes as (source_id, node_id) tuples
    pub fn get_used_source_nodes(&self) -> Vec<(String, String)> {
        fn collect(nodes: &[ContextNode], result: &mut Vec<(String, String)>) {
            for node in nodes {
                match &node.content {
                    NodeContent::Source {
                        source_id, node_id, ..
                    } => {
                        result.push((source_id.clone(), node_id.clone()));
                    }
                    NodeContent::Group { children, .. } => {
                        collect(children, result);
                    }
                    _ => {}
                }
            }
        }

        let mut result = Vec::new();
        collect(&self.nodes, &mut result);
        result
    }

    /// Validate the entire template against a source registry
    pub fn validate(&self, registry: &SourceRegistry) -> Result<(), Vec<TemplateError>> {
        let mut errors = Vec::new();
        let mut used_source_nodes: HashMap<(String, String), Uuid> = HashMap::new();

        fn validate_nodes(
            nodes: &[ContextNode],
            registry: &SourceRegistry,
            used: &mut HashMap<(String, String), Uuid>,
            errors: &mut Vec<TemplateError>,
        ) {
            for node in nodes {
                match &node.content {
                    NodeContent::Source {
                        source_id,
                        node_id,
                        settings: _,
                    } => {
                        // Check for duplicates
                        let key = (source_id.clone(), node_id.clone());
                        if let Some(&_existing_id) = used.get(&key) {
                            errors.push(TemplateError::DuplicateSourceNode {
                                source_id: source_id.clone(),
                                node_id: node_id.clone(),
                            });
                        } else {
                            used.insert(key, node.id);
                        }

                        // Validate source exists
                        match registry.get(source_id) {
                            None => {
                                errors.push(TemplateError::UnknownSource(source_id.clone()));
                            }
                            Some(source) => {
                                // Validate node exists in source
                                if !source.nodes.iter().any(|n| n.node_id == *node_id) {
                                    errors.push(TemplateError::UnknownSourceNode {
                                        source_id: source_id.clone(),
                                        node_id: node_id.clone(),
                                    });
                                }
                                // TODO: validate settings
                            }
                        }
                    }
                    NodeContent::Group { children, .. } => {
                        validate_nodes(children, registry, used, errors);
                    }
                    NodeContent::Text { .. } => {}
                }
            }
        }

        validate_nodes(&self.nodes, registry, &mut used_source_nodes, &mut errors);

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Count total number of nodes in the template
    pub fn node_count(&self) -> usize {
        fn count(nodes: &[ContextNode]) -> usize {
            nodes.iter().fold(0, |acc, node| {
                let child_count = if let NodeContent::Group { children, .. } = &node.content {
                    count(children)
                } else {
                    0
                };
                acc + 1 + child_count
            })
        }
        count(&self.nodes)
    }

    /// Mark the template as updated
    fn mark_updated(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::context::types::PromptRole;

    fn create_test_template() -> ContextTemplate {
        let mut template = ContextTemplate::new("Test Template", Uuid::new_v4());

        // Add some root nodes
        let text_node = ContextNode::new_text("System", "You are helpful", Some(PromptRole::System));
        template.add_node(text_node, None, None).unwrap();

        let mut group = ContextNode::new_group("Character Info");
        if let NodeContent::Group { children, .. } = &mut group.content {
            children.push(ContextNode::new_text("Name", "Alice", None));
            children.push(ContextNode::new_text("Traits", "Kind, smart", None));
        }
        template.add_node(group, None, None).unwrap();

        template
    }

    #[test]
    fn test_find_node_path() {
        let template = create_test_template();

        // Find root node
        let root_node_id = template.nodes[0].id;
        let path = template.find_node_path(root_node_id).unwrap();
        assert_eq!(path, vec![0]);

        // Find nested node
        if let NodeContent::Group { children, .. } = &template.nodes[1].content {
            let nested_id = children[0].id;
            let path = template.find_node_path(nested_id).unwrap();
            assert_eq!(path, vec![1, 0]);
        }
    }

    #[test]
    fn test_add_node_to_root() {
        let mut template = ContextTemplate::new("Test", Uuid::new_v4());
        let node = ContextNode::new_text("Test", "Content", None);
        let node_id = node.id;

        let path = template.add_node(node, None, None).unwrap();
        assert_eq!(path, vec![0]);
        assert_eq!(template.nodes.len(), 1);
        assert_eq!(template.nodes[0].id, node_id);
    }

    #[test]
    fn test_add_node_to_group() {
        let mut template = ContextTemplate::new("Test", Uuid::new_v4());

        // Add a group
        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();

        // Add a node to the group
        let child = ContextNode::new_text("Child", "Content", None);
        let child_id = child.id;
        let path = template.add_node(child, Some(&vec![0]), None).unwrap();

        assert_eq!(path, vec![0, 0]);

        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].id, child_id);
        } else {
            panic!("Expected group");
        }
    }

    #[test]
    fn test_remove_node() {
        let mut template = create_test_template();
        let node_id = template.nodes[0].id;

        let removed = template.remove_node(node_id).unwrap();
        assert_eq!(removed.id, node_id);
        assert_eq!(template.nodes.len(), 1);
    }

    #[test]
    fn test_move_node() {
        let mut template = ContextTemplate::new("Test", Uuid::new_v4());

        // Add a group and a text node
        let group = ContextNode::new_group("Group");
        template.add_node(group, None, None).unwrap();

        let text = ContextNode::new_text("Text", "Content", None);
        let text_id = text.id;
        template.add_node(text, None, None).unwrap();

        // Move text into group
        let new_path = template.move_node(text_id, Some(&vec![0]), 0).unwrap();
        assert_eq!(new_path, vec![0, 0]);

        // Verify it's in the group
        if let NodeContent::Group { children, .. } = &template.nodes[0].content {
            assert_eq!(children.len(), 1);
            assert_eq!(children[0].id, text_id);
        } else {
            panic!("Expected group");
        }

        // Root should only have the group
        assert_eq!(template.nodes.len(), 1);
    }

    #[test]
    fn test_toggle_node() {
        let mut template = create_test_template();
        let node_id = template.nodes[0].id;

        assert!(template.nodes[0].enabled);

        let new_state = template.toggle_node(node_id).unwrap();
        assert!(!new_state);
        assert!(!template.nodes[0].enabled);

        let new_state = template.toggle_node(node_id).unwrap();
        assert!(new_state);
    }

    #[test]
    fn test_duplicate_source_node_prevention() {
        let mut template = ContextTemplate::new("Test", Uuid::new_v4());

        let source1 = ContextNode::new_source("Chat History", "chat_history", "main");
        template.add_node(source1, None, None).unwrap();

        let source2 = ContextNode::new_source("Chat History 2", "chat_history", "main");
        let result = template.add_node(source2, None, None);

        assert!(matches!(result, Err(TemplateError::DuplicateSourceNode { .. })));
    }

    #[test]
    fn test_node_count() {
        let template = create_test_template();
        // 1 root text + 1 group + 2 children = 4
        assert_eq!(template.node_count(), 4);
    }
}
