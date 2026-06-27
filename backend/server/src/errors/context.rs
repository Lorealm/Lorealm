use std::fmt::Display;

use context_weaver::ContextWeaverError;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::{
    errors::{LorebookError, StateError, TokenizerError},
    services::context::NodePath,
};

#[derive(Debug)]
pub enum ContextType {
    Template,
    Lorebook,
    LorebookEntry,
}

impl Display for ContextType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextType::Template => write!(f, "Template"),
            ContextType::Lorebook => write!(f, "Lorebook"),
            ContextType::LorebookEntry => write!(f, "Lorebook Entry"),
        }
    }
}

#[derive(Debug, Error)]
pub enum ContextError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Serialization error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),
    #[error("Lorebook error: {0}")]
    Lorebook(#[from] LorebookError),
    #[error("State error: {0}")]
    State(#[from] StateError),
    #[error("ContextWeaver error: {0}")]
    ContextWeaver(#[from] ContextWeaverError),
    #[error("ContextWeaver error (plugin): {0}")]
    ContextWeaverPlugin(#[from] context_weaver::plugin::PluginError),
    #[error("{0} not found: {1}")]
    NotFound(ContextType, Snowflake),
    #[error("Resolver not found: {0}")]
    ResolverNotFound(String),
    #[error("Resolver found no active character")]
    NoActiveCharacter,
    #[error("Unknown source node: {source_id}:{node_id}")]
    UnknownSourceNode { source_id: String, node_id: String },
    #[error("State conversion: {0}")]
    StateConversion(String),
    #[error("Tokenizer: {0}")]
    Tokenizer(#[from] TokenizerError),
}

impl ContextError {
    pub fn client_message(&self) -> String {
        match self {
            ContextError::Storage(_) | ContextError::Serialization(_) | ContextError::Yaml(_) => {
                "Unknown error".to_string()
            }
            ContextError::NotFound(ctx_type, snowflake) => {
                format!("{ctx_type} not found: {snowflake}")
            }
            ContextError::Template(template_error) => template_error.client_message(),
            ContextError::Lorebook(lorebook_error) => lorebook_error.client_message(),
            ContextError::ResolverNotFound(resolver) => format!("resolver not found: {resolver}"),
            ContextError::State(state_error) => state_error.client_message(),
            ContextError::NoActiveCharacter => "resolver found no active character".to_string(),
            ContextError::UnknownSourceNode { source_id, node_id } => {
                format!("Unknown source node: {source_id}:{node_id}")
            }
            ContextError::ContextWeaver(context_weaver_error) => {
                format!("ContextWeaver: {context_weaver_error}")
            }
            ContextError::ContextWeaverPlugin(context_weaver_error) => {
                format!("ContextWeaver (plugin): {context_weaver_error}")
            }
            ContextError::StateConversion(e) => {
                format!("State conversion: {e}")
            }
            ContextError::Tokenizer(tokenizer_error) => tokenizer_error.client_message(),
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
