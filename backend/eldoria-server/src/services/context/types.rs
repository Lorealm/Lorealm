use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Defines what kind of node this is and its type-specific data
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodeContent {
    /// User-editable text prompt
    Text {
        content: String,
        /// Optional: role hint for the LLM (system, user, assistant)
        role: Option<PromptRole>,
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
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PromptRole {
    System,
    User,
    Assistant,
}

/// A single node in the context template
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextNode {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub enabled: bool,
    pub content: NodeContent,
}

impl ContextNode {
    /// Create a new text node
    pub fn new_text(title: impl Into<String>, content: impl Into<String>, role: Option<PromptRole>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: String::new(),
            enabled: true,
            content: NodeContent::Text {
                content: content.into(),
                role,
            },
        }
    }

    /// Create a new source node
    pub fn new_source(
        title: impl Into<String>,
        source_id: impl Into<String>,
        node_id: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: String::new(),
            enabled: true,
            content: NodeContent::Source {
                source_id: source_id.into(),
                node_id: node_id.into(),
                settings: HashMap::new(),
            },
        }
    }

    /// Create a new group node
    pub fn new_group(title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            title: title.into(),
            description: String::new(),
            enabled: true,
            content: NodeContent::Group {
                children: Vec::new(),
                prefix: None,
                suffix: None,
                always_include_wrappers: false,
            },
        }
    }
}

/// The full template definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextTemplate {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    /// The user who owns this template
    pub owner_id: Uuid,
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
    pub fn new(name: impl Into<String>, owner_id: Uuid) -> Self {
        let now = chrono::Utc::now();
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            owner_id,
            nodes: Vec::new(),
            version: Self::CURRENT_VERSION,
            created_at: now,
            updated_at: now,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_content_serialization() {
        let text_node = NodeContent::Text {
            content: "Hello".into(),
            role: Some(PromptRole::System),
        };

        let json = serde_json::to_string(&text_node).unwrap();
        let deserialized: NodeContent = serde_json::from_str(&json).unwrap();
        assert_eq!(text_node, deserialized);
    }

    #[test]
    fn test_context_node_creation() {
        let node = ContextNode::new_text("Test", "Content", Some(PromptRole::User));
        assert_eq!(node.title, "Test");
        assert!(node.enabled);

        if let NodeContent::Text { content, role } = &node.content {
            assert_eq!(content, "Content");
            assert_eq!(role, &Some(PromptRole::User));
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_template_creation() {
        let owner_id = Uuid::new_v4();
        let template = ContextTemplate::new("My Template", owner_id);

        assert_eq!(template.name, "My Template");
        assert_eq!(template.owner_id, owner_id);
        assert_eq!(template.version, ContextTemplate::CURRENT_VERSION);
        assert!(template.nodes.is_empty());
    }
}