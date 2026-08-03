//! Application-layer error (docs/05 §8).

use thiserror::Error;

/// Application-layer error.
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ApplicationError {
    #[error("not_found: {message}")]
    NotFound { message: String },
    #[error("precondition: {message}")]
    Precondition { message: String },
    #[error("idempotency: {message}")]
    Idempotency { message: String },
    #[error("dependency: {message}")]
    Dependency { message: String },
    #[error("domain: {0}")]
    Domain(enginer_domain::DomainError),
    #[error("security: {0}")]
    Security(String),
}

impl From<enginer_domain::DomainError> for ApplicationError {
    fn from(e: enginer_domain::DomainError) -> Self {
        Self::Domain(e)
    }
}
