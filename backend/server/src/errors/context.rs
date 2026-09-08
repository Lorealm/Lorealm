use std::{borrow::Cow, fmt::Display};

use axum::http::StatusCode;
use context_weaver::ContextWeaverError;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::{
    errors::{
        DomainError, ErrorDetails, LorebookError, StateError, TargetType, TokenizerError,
        impl_app_error, sentence,
    },
    services::context::NodePath,
};

#[derive(Debug, Clone, Copy)]
pub enum ContextType {
    Template,
    Lorebook,
    LorebookEntry,
}

impl ContextType {
    fn target(&self) -> TargetType {
        match self {
            ContextType::Template => TargetType::ContextTemplate,
            ContextType::Lorebook => TargetType::Lorebook,
            ContextType::LorebookEntry => TargetType::LorebookEntry,
        }
    }
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
    StateConversion(#[from] crate::util::weaver::ConversionError),
    #[error("Tokenizer: {0}")]
    Tokenizer(#[from] TokenizerError),
    #[error("No active context template")]
    NoActiveTemplate,
}

impl DomainError for ContextError {
    fn code(&self) -> &'static str {
        match self {
            Self::Template(err) => err.code(),
            Self::Lorebook(err) => err.code(),
            Self::State(err) => err.code(),
            Self::Tokenizer(err) => err.code(),
            Self::NotFound(_, _) => "context.not_found",
            Self::ResolverNotFound(_) => "context.resolver_not_found",
            Self::NoActiveCharacter => "context.no_active_character",
            Self::NoActiveTemplate => "context.no_active_template",
            Self::UnknownSourceNode { .. } => "context.unknown_source_node",
            Self::ContextWeaver(_) => "context.evaluation_failed",
            Self::ContextWeaverPlugin(_) => "context.plugin_failed",
            Self::Storage(_)
            | Self::Serialization(_)
            | Self::Yaml(_)
            | Self::StateConversion(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Template(err) => err.status(),
            Self::Lorebook(err) => err.status(),
            Self::State(err) => err.status(),
            Self::Tokenizer(err) => err.status(),
            Self::NotFound(_, _) => StatusCode::NOT_FOUND,
            Self::NoActiveCharacter => StatusCode::CONFLICT,
            Self::NoActiveTemplate => StatusCode::CONFLICT,
            Self::ResolverNotFound(_)
            | Self::UnknownSourceNode { .. }
            | Self::ContextWeaver(_)
            | Self::ContextWeaverPlugin(_) => StatusCode::BAD_REQUEST,
            Self::Storage(_)
            | Self::Serialization(_)
            | Self::Yaml(_)
            | Self::StateConversion(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Template(err) => err.message(),
            Self::Lorebook(err) => err.message(),
            Self::State(err) => err.message(),
            Self::Tokenizer(err) => err.message(),
            Self::NotFound(kind, _) => {
                Cow::Owned(format!("That {} no longer exists.", kind.target().label()))
            }
            Self::ResolverNotFound(name) => {
                Cow::Owned(format!("Unknown resolver '{name}' in this template."))
            }
            Self::NoActiveCharacter => "No character is selected.".into(),
            Self::NoActiveTemplate => "No template selected".into(),
            Self::UnknownSourceNode { source_id, node_id } => Cow::Owned(format!(
                "This template references an unknown node '{source_id}:{node_id}'."
            )),
            // Both describe the user's own template content.
            Self::ContextWeaver(err) => sentence(format!("Context could not be built: {err}")),
            Self::ContextWeaverPlugin(err) => sentence(format!("A context plugin failed: {err}")),
            Self::Storage(_)
            | Self::Serialization(_)
            | Self::Yaml(_)
            | Self::StateConversion(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::Template(err) => err.details(),
            Self::Lorebook(err) => err.details(),
            Self::State(err) => err.details(),
            Self::Tokenizer(err) => err.details(),
            Self::NotFound(kind, id) => Some(ErrorDetails::target(kind.target(), *id)),
            Self::NoActiveCharacter => Some(ErrorDetails::resource(TargetType::Character)),
            Self::NoActiveTemplate => Some(ErrorDetails::resource(TargetType::ContextTemplate)),
            Self::ResolverNotFound(_)
            | Self::UnknownSourceNode { .. }
            | Self::ContextWeaver(_)
            | Self::ContextWeaverPlugin(_) => {
                Some(ErrorDetails::resource(TargetType::ContextTemplate))
            }
            Self::Storage(_)
            | Self::Serialization(_)
            | Self::Yaml(_)
            | Self::StateConversion(_) => None,
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

    #[error("Circular reference detected")]
    CircularReference,

    #[error("Cannot move node: would create circular reference")]
    WouldCreateCircle,

    #[error("Index out of bounds: {index} >= {len}")]
    IndexOutOfBounds { index: usize, len: usize },
    #[error("Invalid path: {0:?}")]
    InvalidPath(NodePath),
}

impl DomainError for TemplateError {
    fn code(&self) -> &'static str {
        match self {
            Self::NodeNotFound(_) => "template.node_not_found",
            Self::DuplicateSourceNode { .. } => "template.duplicate_source_node",
            Self::InvalidParent(_) => "template.invalid_parent",
            Self::CircularReference | Self::WouldCreateCircle => "template.circular_reference",
            Self::IndexOutOfBounds { .. } => "template.index_out_of_bounds",
            Self::InvalidPath(_) => "template.invalid_path",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NodeNotFound(_) => StatusCode::NOT_FOUND,
            Self::DuplicateSourceNode { .. } => StatusCode::CONFLICT,
            Self::InvalidParent(_)
            | Self::CircularReference
            | Self::WouldCreateCircle
            | Self::IndexOutOfBounds { .. }
            | Self::InvalidPath(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NodeNotFound(_) => "That template node no longer exists.".into(),
            Self::DuplicateSourceNode { source_id, node_id } => Cow::Owned(format!(
                "Node '{source_id}:{node_id}' is already used in this template."
            )),
            Self::InvalidParent(_) => "Nodes can only be nested inside a group.".into(),
            Self::CircularReference | Self::WouldCreateCircle => {
                "That move would create a circular reference.".into()
            }
            Self::IndexOutOfBounds { index, len } => {
                Cow::Owned(format!("Position {index} is out of range (max {len})."))
            }
            Self::InvalidPath(_) => "That template path is not valid.".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NodeNotFound(id) | Self::InvalidParent(id) => {
                Some(ErrorDetails::target(TargetType::TemplateNode, *id))
            }
            _ => Some(ErrorDetails::resource(TargetType::ContextTemplate)),
        }
    }
}

impl_app_error!(ContextError, TemplateError);
