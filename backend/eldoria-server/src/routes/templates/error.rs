use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use thiserror::Error;

use crate::{models::TemplateServiceError, services::context::{EvaluationError, TemplateError}};

use super::dto::ErrorResponse;

/// API error type for template routes
#[derive(Debug, Error)]
pub enum TemplateApiError {
    #[error("Template error: {0}")]
    Template(#[from] TemplateError),

    #[error("Service error: {0}")]
    Service(#[from] TemplateServiceError),

    #[error("Evaluation error: {0}")]
    Evaluation(#[from] EvaluationError),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for TemplateApiError {
    fn into_response(self) -> Response {
        let (status, error_response) = match &self {
            TemplateApiError::Template(e) => match e {
                TemplateError::NodeNotFound(_) => (
                    StatusCode::NOT_FOUND,
                    ErrorResponse::new(e.to_string(), "NODE_NOT_FOUND"),
                ),
                TemplateError::DuplicateSourceNode { .. } => (
                    StatusCode::CONFLICT,
                    ErrorResponse::new(e.to_string(), "DUPLICATE_SOURCE_NODE"),
                ),
                TemplateError::InvalidParent(_) => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "INVALID_PARENT"),
                ),
                TemplateError::UnknownSource(_) => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "UNKNOWN_SOURCE"),
                ),
                TemplateError::UnknownSourceNode { .. } => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "UNKNOWN_SOURCE_NODE"),
                ),
                TemplateError::InvalidSetting { .. } => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "INVALID_SETTING"),
                ),
                TemplateError::CircularReference => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "CIRCULAR_REFERENCE"),
                ),
                TemplateError::WouldCreateCircle => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "WOULD_CREATE_CIRCLE"),
                ),
                TemplateError::IndexOutOfBounds { .. } => (
                    StatusCode::BAD_REQUEST,
                    ErrorResponse::new(e.to_string(), "INDEX_OUT_OF_BOUNDS"),
                ),
            },

            TemplateApiError::Service(e) => match e {
                TemplateServiceError::NotFound(_) => (
                    StatusCode::NOT_FOUND,
                    ErrorResponse::new(e.to_string(), "TEMPLATE_NOT_FOUND"),
                ),
                TemplateServiceError::AccessDenied { .. } => (
                    StatusCode::FORBIDDEN,
                    ErrorResponse::new(e.to_string(), "ACCESS_DENIED"),
                ),
                TemplateServiceError::DuplicateName(_) => (
                    StatusCode::CONFLICT,
                    ErrorResponse::new(e.to_string(), "DUPLICATE_NAME"),
                ),
                TemplateServiceError::Storage(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse::new("Storage error", "STORAGE_ERROR"),
                ),
                TemplateServiceError::Serialization(_) | TemplateServiceError::Deserialization(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse::new("Data format error", "DATA_FORMAT_ERROR"),
                ),
            },

            TemplateApiError::Evaluation(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new(e.to_string(), "EVALUATION_ERROR"),
            ),

            TemplateApiError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse::new(msg, "VALIDATION_ERROR"),
            ),

            TemplateApiError::BadRequest(msg) => (
                StatusCode::BAD_REQUEST,
                ErrorResponse::new(msg, "BAD_REQUEST"),
            ),

            TemplateApiError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                ErrorResponse::new(msg, "NOT_FOUND"),
            ),

            TemplateApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorResponse::new("Authentication required", "UNAUTHORIZED"),
            ),

            TemplateApiError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                ErrorResponse::new(msg, "FORBIDDEN"),
            ),

            TemplateApiError::Internal(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorResponse::new(msg, "INTERNAL_ERROR"),
            ),
        };

        // Log server errors
        if status.is_server_error() {
            tracing::error!("API error: {:?}", self);
        }

        (status, Json(error_response)).into_response()
    }
}

/// Extension trait for converting Results to TemplateApiError
pub trait ResultExt<T> {
    fn bad_request(self) -> Result<T, TemplateApiError>;
    fn not_found(self) -> Result<T, TemplateApiError>;
    fn internal(self) -> Result<T, TemplateApiError>;
}

impl<T, E: std::fmt::Display> ResultExt<T> for Result<T, E> {
    fn bad_request(self) -> Result<T, TemplateApiError> {
        self.map_err(|e| TemplateApiError::BadRequest(e.to_string()))
    }

    fn not_found(self) -> Result<T, TemplateApiError> {
        self.map_err(|e| TemplateApiError::NotFound(e.to_string()))
    }

    fn internal(self) -> Result<T, TemplateApiError> {
        self.map_err(|e| TemplateApiError::Internal(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_template_error_conversion() {
        let error = TemplateApiError::Template(TemplateError::NodeNotFound(Uuid::new_v4()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_service_error_conversion() {
        let error = TemplateApiError::Service(TemplateServiceError::NotFound(Uuid::new_v4()));
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_access_denied_is_forbidden() {
        let error = TemplateApiError::Service(TemplateServiceError::AccessDenied {
            user_id: Uuid::new_v4(),
            template_id: Uuid::new_v4(),
        });
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}