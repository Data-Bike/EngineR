//! Connector errors (docs/14 §4).

use thiserror::Error;

/// Result alias for connector operations.
pub type ConnectorResult<T> = Result<T, ConnectorError>;

/// Error from a connector operation.
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum ConnectorError {
    #[error("credential: {message}")]
    Credential { message: String },

    #[error("rate_limited: retry_after_ms={retry_after_ms}")]
    RateLimited { retry_after_ms: u64 },

    #[error("circuit_open: {message}")]
    CircuitOpen { message: String },

    #[error("timeout: {message}")]
    Timeout { message: String },

    #[error("network: {message}")]
    Network { message: String },

    #[error("parse: {message}")]
    Parse { message: String },

    #[error("mapping: {message}")]
    Mapping { message: String },

    #[error("quota_exceeded: {message}")]
    QuotaExceeded { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("permission_denied: {message}")]
    PermissionDenied { message: String },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },

    #[error("internal: {message}")]
    Internal { message: String },
}
