//! Workflow errors.

use thiserror::Error;

pub type WorkflowResult<T> = Result<T, WorkflowError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum WorkflowError {
    #[error("invalid_transition: {message}")]
    InvalidTransition { message: String },

    #[error("invalid_state: {message}")]
    InvalidState { message: String },

    #[error("guard_failed: {message}")]
    GuardFailed { message: String },

    #[error("task: {message}")]
    Task { message: String },

    #[error("timer: {message}")]
    Timer { message: String },

    #[error("reaction: {message}")]
    Reaction { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("already_exists: {message}")]
    AlreadyExists { message: String },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },
}
