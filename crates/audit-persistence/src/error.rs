//! Audit persistence errors.

use thiserror::Error;

pub type AuditPersistenceResult<T> = Result<T, AuditPersistenceError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum AuditPersistenceError {
    #[error("database: {message}")]
    Database { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("chain_broken: {message}")]
    ChainBroken { message: String },

    #[error("invalid_input: {message}")]
    InvalidInput { message: String },

    #[error("hash: {message}")]
    Hash { message: String },
}
