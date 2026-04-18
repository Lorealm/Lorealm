use eldoria_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::services::context::NodePath;

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    #[error("Template not found: {0}")]
    NotFound(Snowflake),
}

impl ContextError {
    pub fn client_message(&self) -> String {
        match self {
            ContextError::Storage(_) | ContextError::Serialization(_) => {
                "Unknown error".to_string()
            }
            ContextError::NotFound(snowflake) => format!("Template not found: {snowflake}"),
            ContextError::Template(template_error) => template_error.client_message(),
        }
    }
}

#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Node not found: {0}")]
    NodeNotFound(Snowflake),

    #[error("Duplicate source node: source '{source_id}' node '{node_id}' is already used")]
    DuplicateSourceNode { source_id: String, node_id: String },

    #[error("Invalid parent: {0} is not a group node")]
    InvalidParent(Snowflake),

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
    #[error("Invalid path: {0:?}")]
    InvalidPath(NodePath),
}

impl TemplateError {
    pub fn client_message(&self) -> String {
        match self {
            TemplateError::NodeNotFound(id) => format!("Node not found: {id}"),
            TemplateError::DuplicateSourceNode { source_id, node_id } => {
                format!("Duplicate source node: {source_id}:{node_id}")
            }
            TemplateError::InvalidParent(id) => {
                format!("Assigning {id} as a parent is not a valid operation")
            }
            TemplateError::UnknownSource(src) => {
                format!("Source '{src}' was not found in the internal registry")
            }
            TemplateError::UnknownSourceNode { source_id, node_id } => {
                format!("Node '{node_id}' (from '{source_id}') is not a valid node.")
            }
            TemplateError::InvalidSetting {
                source_id,
                node_id,
                key,
            } => format!(
                "Node '{node_id}' (from '{source_id}') does not contain a setting named '{key}'"
            ),
            TemplateError::CircularReference => "Circular reference detected".into(),
            TemplateError::WouldCreateCircle => {
                "This operation is invalid: would create a circular reference".into()
            }
            TemplateError::IndexOutOfBounds { index, len } => {
                format!("Index {index} is out of bounds. (max: {len})")
            }
            TemplateError::InvalidPath(items) => format!("The path '{items:?}' is not valid"),
        }
    }
}
