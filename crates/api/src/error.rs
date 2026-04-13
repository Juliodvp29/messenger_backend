use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use shared::error::DomainError;

use crate::services::error::ServiceError;

#[derive(Debug)]
pub struct ApiError(pub DomainError);

impl From<DomainError> for ApiError {
    fn from(value: DomainError) -> Self {
        Self(value)
    }
}

impl From<ServiceError> for ApiError {
    fn from(value: ServiceError) -> Self {
        Self(value.into())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self.0 {
            DomainError::Validation(msg) => (StatusCode::BAD_REQUEST, msg),
            DomainError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            DomainError::AlreadyExists(msg) => (StatusCode::CONFLICT, msg),
            DomainError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            DomainError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}
