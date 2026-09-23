use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::StorageError;
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin {0} is not installed")]
    NotFound(String),
    #[error("plugin {0} failed to load: {1}")]
    Invalid(String, lorealm_plugins::PluginError),
    /// The reason reads as a predicate: "conflicts with `x`".
    #[error("plugin {0} cannot be enabled: it {1}")]
    CannotEnable(String, String),
    /// Enabled plugins that require this one.
    #[error("plugin {0} is required by {1:?}")]
    Required(String, Vec<String>),
    #[error("load order must list each enabled plugin exactly once")]
    InvalidOrder,
    /// The plugin failed a call made on its frontend's behalf.
    #[error("plugin {0} call failed: {1}")]
    CallFailed(String, String),
    #[error(transparent)]
    Storage(#[from] StorageError),
}

impl DomainError for PluginError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "plugin.not_found",
            Self::Invalid(_, _) => "plugin.invalid",
            Self::CannotEnable(_, _) => "plugin.cannot_enable",
            Self::Required(_, _) => "plugin.required",
            Self::InvalidOrder => "plugin.invalid_order",
            Self::CallFailed(_, _) => "plugin.call_failed",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Invalid(_, _) | Self::InvalidOrder => StatusCode::BAD_REQUEST,
            Self::CannotEnable(_, _) | Self::Required(_, _) => StatusCode::CONFLICT,
            Self::CallFailed(_, _) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotFound(_) => "That plugin is not installed.".into(),
            Self::Invalid(_, _) => "That plugin could not be loaded.".into(),
            Self::CannotEnable(_, reason) => {
                sentence(format!("This plugin can't be enabled: it {reason}"))
            }
            Self::Required(_, by) => {
                let by: Vec<String> = by.iter().map(|h| format!("`{h}`")).collect();
                sentence(format!(
                    "Disable the plugins that require this one first: {}",
                    by.join(", ")
                ))
            }
            Self::InvalidOrder => "The order must list each enabled plugin exactly once.".into(),
            Self::CallFailed(_, _) => {
                "The plugin couldn't complete that request. Its log says why.".into()
            }
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotFound(_)
            | Self::Invalid(_, _)
            | Self::CannotEnable(_, _)
            | Self::Required(_, _)
            | Self::InvalidOrder
            | Self::CallFailed(_, _) => Some(ErrorDetails::resource(TargetType::Plugin)),
            Self::Storage(_) => None,
        }
    }
}

impl_app_error!(PluginError);
