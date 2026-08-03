//! Risk engine errors.

use thiserror::Error;

pub type RiskResult<T> = Result<T, RiskError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum RiskError {
    #[error("ast: {message}")]
    Ast { message: String },

    #[error("evaluation: {message}")]
    Evaluation { message: String },

    #[error("criterion: {message}")]
    Criterion { message: String },

    #[error("factor: {message}")]
    Factor { message: String },

    #[error("score: {message}")]
    Score { message: String },

    #[error("assessment: {message}")]
    Assessment { message: String },

    #[error("decision: {message}")]
    Decision { message: String },

    #[error("type_mismatch: expected {expected}, got {got}")]
    TypeMismatch { expected: String, got: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },
}
