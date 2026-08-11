use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::models::Snowflake;
use thiserror::Error;

use crate::errors::{DomainError, ErrorDetails, TargetType, impl_app_error, sentence};

#[derive(Debug, Error)]
pub enum LorebookError {
    #[error("User {0} does not own this template: {1}")]
    NotOwned(Snowflake, Snowflake),
    #[error("ContextWeaver error: {0}")]
    ContextWeaver(#[from] context_weaver::ContextWeaverError),
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

impl DomainError for LorebookError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotOwned(_, _) => "lorebook.not_found",
            Self::ContextWeaver(_) => "lorebook.evaluation_failed",
            Self::Yaml(_) => "lorebook.invalid_yaml",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::NotOwned(_, _) => StatusCode::NOT_FOUND,
            Self::ContextWeaver(_) | Self::Yaml(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::NotOwned(_, _) => "That lorebook no longer exists.".into(),
            Self::ContextWeaver(err) => sentence(format!("Lorebook could not be evaluated: {err}")),
            Self::Yaml(err) => sentence(format!("Lorebook YAML is invalid: {err}")),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::NotOwned(_, id) => Some(ErrorDetails::target(TargetType::Lorebook, *id)),
            Self::ContextWeaver(_) | Self::Yaml(_) => {
                Some(ErrorDetails::resource(TargetType::Lorebook))
            }
        }
    }
}

impl_app_error!(LorebookError);
