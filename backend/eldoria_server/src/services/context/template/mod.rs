use std::{collections::HashMap, sync::Arc};

use eldoria_storage::models::{Snowflake, Template};
use serde::{Deserialize, Serialize};

use crate::{
    errors::{ContextError, TemplateError},
    services::{context::providers::SourceRegistry, snowflake::SnowflakeService},
};

pub mod providers;

/// The mode for a certain group node.
///
/// Changes the behavior when updating.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupMode {
    /// Makes the group act like a normal container
    Collection,
    /// Makes the group act as a "toggle-group". Only allows the selection of a single item within this group.
    Invariant,
}

/// Defines what kind of node this is and its type-specific data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeContent {
    /// User-editable text prompt
    Text {
        content: String,
        /// Optional: role hint for the LLM (system, user, assistant)
        role: super::PromptRole,
    },

    /// A node provided by a source (chat history, world info, extensions, etc.)
    Source {
        /// Which source provides this node (e.g., "chat_history", "world_info", "ext_summarizer")
        source_id: String,
        /// Which specific node from that source (sources can offer multiple nodes)
        node_id: String,
        /// Source-specific settings
        settings: HashMap<String, serde_json::Value>,
    },

    /// A grouping container for organization
    Group {
        children: Vec<ContextNode>,
        /// Optional text to prepend before group contents
        prefix: Option<String>,
        /// Optional text to append after group contents
        suffix: Option<String>,
        /// Whether prefix/suffix should be included even if all children are disabled
        always_include_wrappers: bool,
        /// Which mode to use for this group
        mode: GroupMode,
    },
}

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
    ) -> Self {
        Self {
            id: snowflake_gen.generate(),
            title: title.into(),
            description: description.into(),
            enabled: true,
            content: NodeContent::Group {
                children,
                prefix: None,
                suffix: None,
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
    /// The user who owns this template
    pub owner_id: Snowflake,
    /// Root-level nodes (can include groups which have their own children)
    pub nodes: Vec<ContextNode>,
    /// Template format version for migrations
    pub version: u32,
    /// When the template was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When the template was last modified
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
                ContextNode::source(
                    sg,
                    "World Info (Prelude)",
                    "Deepest layer of world info",
                    "world_info",
                    "prelude",
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
                ),
                ContextNode::source(
                    sg,
                    "World Info (Preamble)",
                    "Near base of context",
                    "world_info",
                    "preamble",
                ),
                ContextNode::source(
                    sg,
                    "Scenario",
                    "The current scenario",
                    "character",
                    "scenario",
                ),
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
                ContextNode::source(
                    sg,
                    "Example Messages",
                    "Example conversation messages",
                    "character",
                    "example_messages",
                ),
                ContextNode::source(
                    sg,
                    "Chat History",
                    "The conversation history",
                    "chat_history",
                    "main",
                ),
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
                    .ok_or_else(|| TemplateError::NodeNotFound(Snowflake(-1)))?;

                if let NodeContent::Group { children, .. } = &mut parent.content {
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

        // Find the parent's NEW path by ID (if we have a parent)
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
        if let Some(c) = content {
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
        let mut used_source_nodes: HashMap<(String, String), Snowflake> = HashMap::new();

        fn validate_nodes(
            nodes: &[ContextNode],
            registry: &SourceRegistry,
            used: &mut HashMap<(String, String), Snowflake>,
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

/// Path to a node within the template (list of indices at each level)
pub type NodePath = Vec<usize>;

/// Describes a setting that a source node accepts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingDefinition {
    pub key: String,
    pub label: String,
    pub description: String,
    pub setting_type: SettingType,
    pub default_value: serde_json::Value,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SettingType {
    String { max_length: Option<usize> },
    Integer { min: Option<i64>, max: Option<i64> },
    Float { min: Option<f64>, max: Option<f64> },
    Boolean,
    Select { options: Vec<SelectOption> },
    TokenCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

/// Describes a single node that a source can provide
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceNodeDefinition {
    /// Unique ID for this node within the source
    pub node_id: String,
    pub title: String,
    pub description: String,
    /// Icon identifier (e.g., "message-square", "book", "brain")
    pub icon: Option<String>,
    /// Settings this node accepts
    pub settings: Vec<SettingDefinition>,
}

/// Describes a source that can provide context nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDefinition {
    /// Unique identifier for this source
    pub source_id: String,
    pub name: String,
    pub description: String,
    /// Whether this is a built-in source or from an extension
    pub source_type: SourceType,
    /// All nodes this source can provide
    pub nodes: Vec<SourceNodeDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    BuiltIn,
    Extension { extension_id: String },
}
