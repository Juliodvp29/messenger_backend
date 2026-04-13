use thiserror::Error;
use shared::error::DomainError;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl serde::Serialize for ServiceError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<ServiceError> for DomainError {
    fn from(value: ServiceError) -> Self {
        match value {
            ServiceError::Internal(msg) => DomainError::Internal(msg),
            ServiceError::Unauthorized(msg) => DomainError::Unauthorized(msg),
            ServiceError::NotFound(msg) => DomainError::NotFound(msg),
            ServiceError::AlreadyExists(msg) => DomainError::AlreadyExists(msg),
            ServiceError::Validation(msg) => DomainError::Validation(msg),
        }
    }
}
