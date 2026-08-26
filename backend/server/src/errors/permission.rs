use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::StorageError;
use thiserror::Error;

use crate::{
    errors::{DomainError, impl_app_error},
    services::permission::Permissions,
};

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Missing permissions: {0}")]
    Missing(Permissions),
    #[error("Storage Error: {0}")]
    Storage(#[from] StorageError),
}

impl DomainError for PermissionError {
    fn code(&self) -> &'static str {
        match self {
            Self::Missing(_) => "permission.denied",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Missing(_) => StatusCode::FORBIDDEN,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Missing(_) => "You are not allowed to perform this action.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }
}

impl_app_error!(PermissionError);
