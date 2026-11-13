use std::collections::HashMap;

use lorealm_storage::models::Snowflake;
use serde::{Deserialize, Serialize};

use crate::services::{
    context_template::{
        ContextNode, CreateNode, MoveNode, MovedNode, NodeContent, NodePath, NodeWithPath,
        UpdateNode,
        weaver::check::{Diagnostic, Dialect},
    },
    weaver::WeaverProfile,
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

/// Weaver source to check, as the editor currently holds it.
#[derive(Deserialize)]
pub(super) struct WeaverCheckRequest {
    pub source: String,
    /// Defaults to `template`
    #[serde(default)]
    pub dialect: Dialect,
    /// Which host the source will run in.
    #[serde(default)]
    pub profile: Option<WeaverProfile>,
}

/// Problems found in that source. An empty list means it is clean, not
/// that it would evaluate — only the parser and the registries ran.
#[derive(Serialize)]
pub(super) struct WeaverCheckResponse {
    pub diagnostics: Vec<Diagnostic>,
}

/// Every weaver command, processor and namespace an editor can offer
#[derive(Serialize)]
pub(super) struct WeaverVocabulary {
    pub commands: Vec<weaver_lang::registry::CommandSignature>,
    pub processors: Vec<weaver_lang::registry::ProcessorSignature>,
    pub namespaces: Vec<crate::services::weaver::NamespaceDef>,
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

impl From<CreateNodeRequest> for CreateNode {
    fn from(val: CreateNodeRequest) -> Self {
        CreateNode {
            title: val.title,
            description: val.description,
            content: val.content,
            parent_path: val.parent_path,
            index: val.index,
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
