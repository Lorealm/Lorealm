use std::{collections::HashMap, sync::Arc};

use lorealm_storage::models::{Snowflake, Template};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ContextError, TemplateError},
    services::{context::PromptRole, snowflake::SnowflakeService},
    util::template::CORE_INSTRUCTIONS,
};

mod types;

pub use types::*;

/// A single node in the context template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextNode {
    pub id: Snowflake,
    pub title: String,
    pub description: String,
    pub enabled: bool,
    pub content: NodeContent,
}

impl ContextNode {
    pub fn text(
        snowflake_gen: &Arc<SnowflakeService>,
        title: impl Into<String>,
        description: impl Into<String>,
        content: impl Into<String>,
        role: PromptRole,
    ) -> Self {
        Self {
            id: snowflake_gen.generate(),
            title: title.into(),
            description: description.into(),
            enabled: true,
            content: NodeContent::Text {
                content: content.into(),
                role,
            },
        }
    }
    pub fn source(
        snowflake_gen: &Arc<SnowflakeService>,
        title: impl Into<String>,
        description: impl Into<String>,
        source_id: impl Into<String>,
        node_id: impl Into<String>,
    ) -> Self {
        Self {
            id: snowflake_gen.generate(),
            title: title.into(),
            description: description.into(),
            enabled: true,
            content: NodeContent::Source {
                source_id: source_id.into(),
                node_id: node_id.into(),
                settings: HashMap::new(),
            },
        }
    }

    pub fn group(
        snowflake_gen: &Arc<SnowflakeService>,
        title: impl Into<String>,
        description: impl Into<String>,
        children: Vec<ContextNode>,
        wrappers: (Option<String>, Option<String>),
    ) -> Self {
        Self {
            id: snowflake_gen.generate(),
            title: title.into(),
            description: description.into(),
            enabled: true,
            content: NodeContent::Group {
                children,
                wrap_mode: GroupWrapMode::NoMixed,
                prefix: wrappers.0,
                suffix: wrappers.1,
                always_include_wrappers: false,
                mode: GroupMode::Collection,
            },
        }
    }
}

/// The full template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTemplate {
    pub id: Snowflake,
    pub name: String,
    pub description: String,
    pub owner_id: Snowflake,
    /// Root-level nodes
    pub nodes: Vec<ContextNode>,
    /// Template format version for migrations
    pub version: u32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ContextTemplate {
    /// Current template schema version
    pub const CURRENT_VERSION: u32 = 1;

    /// Create a new empty template
    pub fn new(
        name: impl Into<String>,
        owner_id: Snowflake,
        snowflake_gen: &Arc<SnowflakeService>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: snowflake_gen.generate(),
            name: name.into(),
            description: String::new(),
            owner_id,
            nodes: Vec::new(),
            version: Self::CURRENT_VERSION,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn create_default(owner_id: Snowflake, snowflake_gen: &Arc<SnowflakeService>) -> Self {
        let now = chrono::Utc::now();
        let sg = snowflake_gen;

        Self {
            id: sg.generate(),
            name: "Default Template".into(),
            description: "An automatically generated template.".into(),
            owner_id,
            nodes: vec![
                ContextNode::group(
                    sg,
                    "Rules",
                    "The rules the assistant should adhere to",
                    vec![ContextNode::text(
                        sg,
                        "Core Instructions",
                        "The core instructions the model should follow",
                        CORE_INSTRUCTIONS,
                        PromptRole::System,
                    )],
                    (Some("<rules>".into()), Some("</rules>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Persona",
                    "A container for the user/agent's persona",
                    vec![ContextNode::source(
                        sg,
                        "Persona",
                        "The active persona.",
                        "persona",
                        "description",
                    )],
                    (Some("<persona>".into()), Some("</persona>".into())),
                ),
                ContextNode::group(
                    sg,
                    "World Info (before)",
                    "A container for world world info output",
                    vec![
                        ContextNode::source(
                            sg,
                            "World Info (Prelude)",
                            "Deepest layer of world info",
                            "world_info",
                            "prelude",
                        ),
                        ContextNode::source(
                            sg,
                            "World Info (Preamble)",
                            "Near base of context",
                            "world_info",
                            "preamble",
                        ),
                    ],
                    (Some("<world_info>".into()), Some("</world_info>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Character",
                    "Character-related nodes",
                    vec![
                        ContextNode::source(
                            sg,
                            "Definition",
                            "The active character's definition",
                            "character",
                            "definition",
                        ),
                        ContextNode::source(
                            sg,
                            "Personality",
                            "Character personality traits",
                            "character",
                            "personality",
                        ),
                    ],
                    (Some("<character>".into()), Some("</character>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Scenario",
                    "A container for the scenario",
                    vec![ContextNode::source(
                        sg,
                        "Scenario",
                        "The current scenario",
                        "character",
                        "scenario",
                    )],
                    (Some("<scenario>".into()), Some("</scenario>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Author's Note",
                    "A container for the author's note",
                    vec![ContextNode::source(
                        sg,
                        "Author's Note",
                        "Your active author's note",
                        "author_note",
                        "main",
                    )],
                    (
                        Some("<authors_note>".into()),
                        Some("</authors_note>".into()),
                    ),
                ),
                ContextNode::group(
                    sg,
                    "Widgets",
                    "A container for the widget state and directives",
                    vec![
                        ContextNode::group(
                            sg,
                            "Directives",
                            "A container for the widget directives",
                            vec![ContextNode::source(
                                sg,
                                "Widget Directives",
                                "The widgets' instructions",
                                "widgets",
                                "main",
                            )],
                            (Some("<directives>".into()), Some("</directives>".into())),
                        ),
                        ContextNode::group(
                            sg,
                            "State",
                            "A container for the widget states",
                            vec![ContextNode::source(
                                sg,
                                "Widget Directives",
                                "The widgets' states",
                                "widgets",
                                "state",
                            )],
                            (Some("<states>".into()), Some("</states>".into())),
                        ),
                    ],
                    (Some("<scenario>".into()), Some("</scenario>".into())),
                ),
                ContextNode::group(
                    sg,
                    "World Info (middle)",
                    "A container for world world info output",
                    vec![
                        ContextNode::source(
                            sg,
                            "World Info (Backdrop)",
                            "Background context",
                            "world_info",
                            "backdrop",
                        ),
                        ContextNode::source(
                            sg,
                            "World Info (Setting)",
                            "Middle of context",
                            "world_info",
                            "setting",
                        ),
                    ],
                    (Some("<world_info>".into()), Some("</world_info>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Example Messages",
                    "A container for the example messages",
                    vec![ContextNode::source(
                        sg,
                        "Example Messages",
                        "Example conversation messages",
                        "character",
                        "example_messages",
                    )],
                    (Some("<examples>".into()), Some("</examples>".into())),
                ),
                ContextNode::group(
                    sg,
                    "State Template",
                    "A container for the state template",
                    vec![ContextNode::source(
                        sg,
                        "Character Injection",
                        "Text the character's card adds",
                        "character",
                        "injection",
                    )],
                    (Some("<template>".into()), Some("</template>".into())),
                ),
                ContextNode::group(
                    sg,
                    "Chat History",
                    "A container for the chat history",
                    vec![ContextNode::source(
                        sg,
                        "Chat History",
                        "The conversation history",
                        "chat_history",
                        "main",
                    )],
                    (Some("<history>".into()), Some("</history>".into())),
                ),
                ContextNode::group(
                    sg,
                    "World Info (after)",
                    "A container for world world info output",
                    vec![
                        ContextNode::source(
                            sg,
                            "World Info (Foreground)",
                            "Near front of context",
                            "world_info",
                            "foreground",
                        ),
                        ContextNode::source(
                            sg,
                            "World Info (Coda)",
                            "Most recent layer",
                            "world_info",
                            "coda",
                        ),
                    ],
                    (Some("<world_info>".into()), Some("</world_info>".into())),
                ),
            ],
            version: Self::CURRENT_VERSION,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn from_db(template: Template) -> Result<Self, ContextError> {
        Ok(Self {
            id: template.id,
            name: template.name,
            description: template.description,
            owner_id: template.owner_id,
            nodes: serde_json::from_value(template.nodes_json)?,
            version: template.version,
            created_at: template.created_at,
            updated_at: template.updated_at,
        })
    }

    pub fn to_db(&self) -> Result<Template, ContextError> {
        Ok(Template {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            owner_id: self.owner_id,
            nodes_json: serde_json::to_value(&self.nodes)?,
            version: self.version,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl ContextTemplate {
    /// Find a node by ID and return its path
    pub fn find_node_path(&self, node_id: Snowflake) -> Option<NodePath> {
        fn search(nodes: &[ContextNode], target: Snowflake, path: &mut NodePath) -> bool {
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

    /// Get a mutable reference to a node by its ID
    pub fn get_node_by_id_mut(&mut self, node_id: Snowflake) -> Option<&mut ContextNode> {
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

    /// Add a new node to the template
    pub fn add_node(
        &mut self,
        mut node: ContextNode,
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
                    .ok_or_else(|| TemplateError::NodeNotFound(Snowflake(-1)))?;

                if let NodeContent::Group { children, mode, .. } = &mut parent.content {
                    // An invariant group allows only one active child.
                    if *mode == GroupMode::Invariant && children.iter().any(|c| c.enabled) {
                        node.enabled = false;
                    }
                    children
                } else {
                    let parent_id = self.get_node(path).map(|n| n.id).unwrap_or(Snowflake(-1));
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
    pub fn remove_node(&mut self, node_id: Snowflake) -> Result<ContextNode, TemplateError> {
        let path = self
            .find_node_path(node_id)
            .ok_or(TemplateError::NodeNotFound(node_id))?;

        let index = *path.last().unwrap();

        let removed = if path.len() == 1 {
            self.nodes.remove(index)
        } else {
            let parent_path: Vec<usize> = path[..path.len() - 1].to_vec();
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
        node_id: Snowflake,
        new_parent_path: Option<&NodePath>,
        new_index: usize,
    ) -> Result<NodePath, TemplateError> {
        // Resolve parent to Snowflake before we modify the tree
        let new_parent_id: Option<Snowflake> = match new_parent_path {
            Some(path) => {
                let parent_node = self
                    .get_node(path)
                    .ok_or_else(|| TemplateError::NodeNotFound(Snowflake(-1)))?;

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

        let resolved_parent_path: Option<NodePath> = match new_parent_id {
            Some(parent_id) => {
                let path = self
                    .find_node_path(parent_id)
                    .ok_or(TemplateError::NodeNotFound(parent_id))?;
                Some(path)
            }
            None => None,
        };

        // Remove the node (this changes paths in the tree)
        let node = self.remove_node(node_id)?;

        // Now add the node using the resolved path
        self.add_node(node, resolved_parent_path.as_ref(), Some(new_index))
    }

    /// Check if `potential_ancestor` is an ancestor of `node_id`
    fn is_ancestor_of(&self, potential_ancestor: Snowflake, node_id: Snowflake) -> bool {
        fn check_children(
            nodes: &[ContextNode],
            ancestor_id: Snowflake,
            target_id: Snowflake,
        ) -> bool {
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

        fn contains_node(node: &ContextNode, target_id: Snowflake) -> bool {
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
        node_id: Snowflake,
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
        if let Some(mut c) = content {
            // Check whether the group was just switched from [`GroupMode::Collection`] to [`GroupMode::Invariant`]
            let perform_disable = match (&c, &node.content) {
                (
                    NodeContent::Group { mode: new_mode, .. },
                    NodeContent::Group { mode: old_mode, .. },
                ) => new_mode == &GroupMode::Invariant && old_mode != &GroupMode::Invariant,
                _ => false,
            };

            // Find the first non-disabled child node then disable the intersection
            if perform_disable {
                if let NodeContent::Group { children, .. } = &mut c {
                    let first_active_idx = children.iter().position(|child| child.enabled);

                    for (i, child) in children.iter_mut().enumerate() {
                        if Some(i) != first_active_idx {
                            child.enabled = false;
                        }
                    }
                }
            }

            node.content = c;
        }

        self.mark_updated();
        Ok(())
    }

    /// Toggle a node's enabled state
    pub fn toggle_node(&mut self, node_id: Snowflake) -> Result<bool, TemplateError> {
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
        exclude_node_id: Snowflake,
    ) -> bool {
        fn search(
            nodes: &[ContextNode],
            source_id: &str,
            node_id: &str,
            exclude_id: Snowflake,
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

    /// Check if a source node exists in the *active* tree, i.e. the node itself
    /// is enabled and every group above it is enabled too.
    pub fn is_source_node_active(&self, source_id: &str, node_id: &str) -> bool {
        fn search(nodes: &[ContextNode], source_id: &str, node_id: &str) -> bool {
            for node in nodes {
                if !node.enabled {
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

    /// Mark the template as updated
    fn mark_updated(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adding_to_an_invariant_group_keeps_one_child_active() {
        let sf = Arc::new(SnowflakeService::new(1));
        let text = |title: &str| ContextNode::text(&sf, title, "", "", PromptRole::System);

        let mut group = ContextNode::group(&sf, "g", "", vec![text("first")], (None, None));
        if let NodeContent::Group { mode, .. } = &mut group.content {
            *mode = GroupMode::Invariant;
        }
        let mut template = ContextTemplate::new("t", Snowflake(1), &sf);
        template.nodes.push(group);

        let path = template.add_node(text("second"), Some(&vec![0]), None).unwrap();
        assert!(!template.get_node(&path).unwrap().enabled);

        // An empty invariant group still takes its first child as active.
        if let NodeContent::Group { children, .. } = &mut template.nodes[0].content {
            children.clear();
        }
        let path = template.add_node(text("only"), Some(&vec![0]), None).unwrap();
        assert!(template.get_node(&path).unwrap().enabled);
    }
}
