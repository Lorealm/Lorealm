use std::collections::HashMap;

use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::services::context::{
    ContextNode, CreateNode, MoveNode, MovedNode, NodeContent, NodePath, NodeWithPath, UpdateNode,
};

#[derive(Deserialize)]
pub(super) struct CreateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub default: bool,
}

#[derive(Deserialize)]
pub(super) struct SetActiveRequest {
    pub id: Snowflake,
}

#[derive(Deserialize)]
pub(super) struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct CreateNodeRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub content: NodeContent,
    /// Path to parent group (None = root level)
    pub parent_path: Option<NodePath>,
    /// Index within parent (None = append)
    pub index: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(super) struct NodeResponse {
    pub node: ContextNode,
    pub path: NodePath,
}

/// Every weaver command and processor an editor can offer
#[derive(Serialize)]
pub(super) struct WeaverVocabulary {
    pub commands: Vec<weaver_lang::registry::CommandSignature>,
    pub processors: Vec<weaver_lang::registry::ProcessorSignature>,
}

#[derive(Debug, Deserialize)]
pub(super) struct UpdateNodeRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub content: Option<NodeContent>,
}

#[derive(Debug, Deserialize)]
pub(super) struct MoveNodeRequest {
    pub new_parent_path: Option<NodePath>,
    pub new_index: usize,
}

#[derive(Debug, Serialize)]
pub(super) struct MoveNodeResponse {
    pub new_path: NodePath,
}

#[derive(Debug, Serialize)]
pub(super) struct ToggleNodeResponse {
    pub delta: HashMap<Snowflake, bool>,
}

impl Into<CreateNode> for CreateNodeRequest {
    fn into(self) -> CreateNode {
        CreateNode {
            title: self.title,
            description: self.description,
            content: self.content,
            parent_path: self.parent_path,
            index: self.index,
        }
    }
}

impl From<UpdateNodeRequest> for UpdateNode {
    fn from(val: UpdateNodeRequest) -> Self {
        UpdateNode {
            title: val.title,
            description: val.description,
            enabled: val.enabled,
            content: val.content,
        }
    }
}

impl From<MoveNodeRequest> for MoveNode {
    fn from(val: MoveNodeRequest) -> Self {
        MoveNode {
            new_parent_path: val.new_parent_path,
            new_index: val.new_index,
        }
    }
}

impl From<NodeWithPath> for NodeResponse {
    fn from(value: NodeWithPath) -> Self {
        Self {
            node: value.node,
            path: value.path,
        }
    }
}

impl From<MovedNode> for MoveNodeResponse {
    fn from(value: MovedNode) -> Self {
        Self {
            new_path: value.new_path,
        }
    }
}
