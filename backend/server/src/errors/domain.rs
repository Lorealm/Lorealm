use std::borrow::Cow;

use axum::http::StatusCode;
use lorealm_storage::models::Snowflake;
use serde::Serialize;

use crate::errors::TargetType;

/// Machine-readable pointer to the resource a failure concerns.
#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resource: Option<TargetType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Snowflake>,
}

impl ErrorDetails {
    /// A resource kind with no specific instance, e.g. "no persona is selected".
    pub fn resource(resource: TargetType) -> Self {
        Self {
            resource: Some(resource),
            id: None,
        }
    }

    /// A specific resource instance.
    pub fn target(resource: TargetType, id: Snowflake) -> Self {
        Self {
            resource: Some(resource),
            id: Some(id),
        }
    }
}

/// Maps an error variant to its wire representation.
pub trait DomainError: std::error::Error {
    /// Stable `domain.reason` identifier.
    fn code(&self) -> &'static str;

    fn status(&self) -> StatusCode;

    /// User-facing sentence.
    fn message(&self) -> Cow<'static, str>;

    fn details(&self) -> Option<ErrorDetails> {
        None
    }
}

/// Punctuates a message that ends in interpolated text.
pub fn sentence(text: String) -> Cow<'static, str> {
    let trimmed = text.trim_end();
    if trimmed.ends_with(['.', '!', '?']) {
        Cow::Owned(trimmed.to_owned())
    } else {
        Cow::Owned(format!("{trimmed}."))
    }
}

/// Generates `From<$ty> for AppError` for each domain error.
///
/// A blanket impl over [`DomainError`] would collide with core's reflexive
/// `impl<T> From<T> for T`, so the impls are generated per type.
macro_rules! impl_app_error {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for $crate::errors::AppError {
                fn from(err: $ty) -> Self {
                    $crate::errors::AppError::from_domain(&err)
                }
            }
        )*
    };
}

pub(crate) use impl_app_error;
