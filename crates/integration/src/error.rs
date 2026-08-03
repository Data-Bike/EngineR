//! Integration errors.

use thiserror::Error;

pub type IntegrationResult<T> = Result<T, IntegrationError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum IntegrationError {
    #[error("check_flow: {message}")]
    CheckFlow { message: String },

    #[error("connector: {message}")]
    Connector { message: String },

    #[error("risk_engine: {message}")]
    RiskEngine { message: String },

    #[error("workflow: {message}")]
    Workflow { message: String },

    #[error("security: {message}")]
    Security { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("invalid_state: {message}")]
    InvalidState { message: String },

    #[error("port: {message}")]
    Port { message: String },
}
