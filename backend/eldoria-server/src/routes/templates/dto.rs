use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::services::context::{
    ContextNode, ContextTemplate, NodeContent, NodePath, SourceDefinition,
};

// ==================== Request DTOs ====================

#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateNodeRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub content: NodeContent,
    /// Path to parent group (None = root level)
    pub parent_path: Option<NodePath>,
    /// Index within parent (None = append)
    pub index: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateNodeRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub enabled: Option<bool>,
    pub content: Option<NodeContent>,
}

#[derive(Debug, Deserialize)]
pub struct MoveNodeRequest {
    pub new_parent_path: Option<NodePath>,
    pub new_index: usize,
}

#[derive(Debug, Deserialize)]
pub struct PreviewContextRequest {
    /// Optional chat ID to use for evaluation
    pub chat_id: Option<Uuid>,
    /// Optional character ID to use for evaluation
    pub character_id: Option<Uuid>,
}

// ==================== Response DTOs ====================

#[derive(Debug, Serialize)]
pub struct TemplateListResponse {
    pub templates: Vec<TemplateSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct TemplateSummary {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub node_count: usize,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&ContextTemplate> for TemplateSummary {
    fn from(template: &ContextTemplate) -> Self {
        Self {
            id: template.id,
            name: template.name.clone(),
            description: template.description.clone(),
            node_count: template.node_count(),
            created_at: template.created_at,
            updated_at: template.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TemplateResponse {
    pub template: ContextTemplate,
    pub available_sources: Vec<SourceDefinition>,
    pub used_source_nodes: Vec<(String, String)>,
}

#[derive(Debug, Serialize)]
pub struct NodeResponse {
    pub node: ContextNode,
    pub path: NodePath,
}

#[derive(Debug, Serialize)]
pub struct MoveNodeResponse {
    pub new_path: NodePath,
}

#[derive(Debug, Serialize)]
pub struct ToggleNodeResponse {
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct SourcesResponse {
    pub sources: Vec<SourceDefinition>,
}

#[derive(Debug, Serialize)]
pub struct ValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub segments: Vec<PreviewSegment>,
    pub total_tokens: usize,
    pub combined_text: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewSegment {
    pub source_type: String,
    pub source_id: Option<String>,
    pub content: String,
    pub role: Option<String>,
    pub token_count: usize,
}

// ==================== Error Response ====================

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, code: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            code: code.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}