//! Domain errors (docs/05 §8).
//!
//! Each layer has its own error enum; this is the domain layer. Domain errors
//! are validation/conflict/invariant failures only — no SQL, no HTTP status.

use thiserror::Error;

/// Domain-layer error.
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum DomainError {
    /// A value failed domain validation.
    #[error("validation: {message}")]
    Validation { message: String },

    /// An invariant was violated (e.g. statement with both entity and scalar object).
    #[error("invariant: {message}")]
    Invariant { message: String },

    /// A semantic conflict (e.g. identity collision, type membership conflict).
    #[error("conflict: {message}")]
    Conflict { message: String },

    /// A referenced entity/type/predicate does not exist or is retired.
    #[error("not_found: {message}")]
    NotFound { message: String },

    /// A schema revision is not in the required state for the operation.
    #[error("schema_state: {message}")]
    SchemaState { message: String },

    /// A normalization/conversion produced an ambiguous or invalid result.
    #[error("conversion: {message}")]
    Conversion { message: String },
}

impl DomainError {
    /// Convenience constructor.
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation {
            message: msg.into(),
        }
    }

    /// Convenience constructor.
    pub fn invariant(msg: impl Into<String>) -> Self {
        Self::Invariant {
            message: msg.into(),
        }
    }

    /// Convenience constructor.
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict {
            message: msg.into(),
        }
    }

    /// Convenience constructor.
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound {
            message: msg.into(),
        }
    }
}
