//! Security errors.

use thiserror::Error;

pub type SecurityResult<T> = Result<T, SecurityError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SecurityError {
    #[error("denied: {message}")]
    Denied { message: String },

    #[error("pdp_unavailable: {message}")]
    PdpUnavailable { message: String },

    #[error("audit_unavailable: {message}")]
    AuditUnavailable { message: String },

    #[error("policy_stale: {message}")]
    PolicyStale { message: String },

    #[error("capability_missing: {capability}")]
    CapabilityMissing { capability: String },

    #[error("classification_violation: {message}")]
    ClassificationViolation { message: String },

    #[error("break_glass_required: {message}")]
    BreakGlassRequired { message: String },

    #[error("step_up_required: {message}")]
    StepUpRequired { message: String },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },
}
