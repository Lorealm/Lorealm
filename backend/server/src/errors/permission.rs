use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::{StorageError, models::Snowflake};
use thiserror::Error;

use crate::{
    errors::{DomainError, ErrorDetails, TargetType, impl_app_error},
    services::permission::Permissions,
};

#[derive(Debug, Error)]
pub enum PermissionError {
    #[error("Missing permissions: {0}")]
    Missing(Permissions),
    #[error("Role not found: {0}")]
    RoleNotFound(Snowflake),
    #[error("User not found")]
    UserNotFound,
    #[error("Role is not below the actor's highest role")]
    Outranked,
    #[error("Built-in role cannot be changed this way")]
    BuiltIn,
    #[error("Role must keep at least one member")]
    LastAdministrator,
    #[error("Role name taken")]
    NameTaken,
    #[error("Invalid role name")]
    InvalidName,
    #[error("Invalid role position")]
    InvalidPosition,
    #[error("Storage Error: {0}")]
    Storage(#[from] StorageError),
}

impl DomainError for PermissionError {
    fn code(&self) -> &'static str {
        match self {
            Self::Missing(_) => "permission.denied",
            Self::RoleNotFound(_) => "permission.role_not_found",
            Self::UserNotFound => "permission.user_not_found",
            Self::Outranked => "permission.role_outranked",
            Self::BuiltIn => "permission.built_in_role",
            Self::LastAdministrator => "permission.last_administrator",
            Self::NameTaken => "permission.role_name_taken",
            Self::InvalidName => "permission.invalid_role_name",
            Self::InvalidPosition => "permission.invalid_role_position",
            Self::Storage(_) => "internal",
        }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Missing(_) | Self::Outranked => StatusCode::FORBIDDEN,
            Self::RoleNotFound(_) | Self::UserNotFound => StatusCode::NOT_FOUND,
            Self::BuiltIn | Self::InvalidName | Self::InvalidPosition => StatusCode::BAD_REQUEST,
            Self::LastAdministrator | Self::NameTaken => StatusCode::CONFLICT,
            Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> Cow<'static, str> {
        match self {
            Self::Missing(_) => "You are not allowed to perform this action.".into(),
            Self::RoleNotFound(_) => "That role no longer exists.".into(),
            Self::UserNotFound => "No user has that name.".into(),
            Self::Outranked => "You can only manage roles below your highest role.".into(),
            Self::BuiltIn => "Built-in roles can't be changed this way.".into(),
            Self::LastAdministrator => "The server needs at least one administrator.".into(),
            Self::NameTaken => "A role with that name already exists.".into(),
            Self::InvalidName => "Role names can't be empty.".into(),
            Self::InvalidPosition => "Role positions start at 1.".into(),
            Self::Storage(_) => "An internal error occurred".into(),
        }
    }

    fn details(&self) -> Option<ErrorDetails> {
        match self {
            Self::RoleNotFound(id) => Some(ErrorDetails::target(TargetType::Role, *id)),
            Self::UserNotFound => Some(ErrorDetails::resource(TargetType::User)),
            _ => None,
        }
    }
}

impl_app_error!(PermissionError);
