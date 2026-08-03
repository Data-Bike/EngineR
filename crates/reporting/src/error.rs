//! Reporting errors.

use thiserror::Error;

pub type ReportResult<T> = Result<T, ReportError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ReportError {
    #[error("dataset: {message}")]
    Dataset { message: String },

    #[error("definition: {message}")]
    Definition { message: String },

    #[error("template: {message}")]
    Template { message: String },

    #[error("render: {message}")]
    Render { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("permission_denied: {message}")]
    PermissionDenied { message: String },

    #[error("limit_exceeded: {message}")]
    LimitExceeded { message: String },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },
}
