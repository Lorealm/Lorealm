use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::services::context_template::{ContextNode, PromptRole};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GroupWrapMode {
    /// Only creates wrappers if all [`PromptRole`]s are the same
    NoMixed,
    /// Wraps around the most abundant [`PromptRole`]
    Majority,
    /// Creates new wrapper groups for each prompt role
    All,
    /// Only create for this specific prompt role
    Only(PromptRole),
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
        wrap_mode: GroupWrapMode,
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
    Text,
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
