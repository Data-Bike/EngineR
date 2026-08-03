//! Stable API error codes (docs/06 §2, docs/07 §13, docs/09 §10).
//!
//! The external response never contains SQL, stack traces, secrets or the fact
//! of existence of a hidden object (docs/05 §8). `message_key` localizes on the
//! Nuxt side; `code`/`parameters` are normative.

use serde::{Deserialize, Serialize};

/// Stable, normative error code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorCode {
    // Generic
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    PreconditionFailed,
    UnprocessableEntity,
    TooManyRequests,
    InternalError,
    ServiceUnavailable,
    // Domain (docs/03 §12, docs/04)
    EntityVersionConflict,
    IdentityConflict,
    IdempotencyConflict,
    // Schema (docs/12)
    SchemaStateInvalid,
    SchemaValidationFailed,
    // Query (docs/07 §13)
    QuerySyntaxInvalid,
    QueryUnknownType,
    QueryUnknownPredicate,
    QueryOperatorNotSupported,
    QueryFieldForbidden,
    QueryTooComplex,
    QueryRequiresBatch,
    QueryCursorInvalid,
    QueryCursorStale,
    QueryProjectionLag,
    QueryTimeout,
    // Security (docs/09 §10)
    SecurityUnauthenticated,
    SecurityCapabilityDenied,
    SecurityDataDenied,
    SecurityStepUpRequired,
    SecurityPolicyStale,
    SecurityPdpUnavailable,
    SecurityContextInvalid,
}

impl ApiErrorCode {
    /// HTTP status for this code (docs/06 §3).
    pub fn http_status(&self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::Unauthorized | Self::SecurityUnauthenticated => 401,
            Self::Forbidden | Self::SecurityCapabilityDenied => 403,
            Self::NotFound | Self::SecurityDataDenied => 404,
            Self::Conflict | Self::IdentityConflict | Self::IdempotencyConflict => 409,
            Self::PreconditionFailed | Self::EntityVersionConflict => 412,
            Self::UnprocessableEntity
            | Self::SchemaStateInvalid
            | Self::SchemaValidationFailed
            | Self::QuerySyntaxInvalid
            | Self::QueryUnknownType
            | Self::QueryUnknownPredicate
            | Self::QueryOperatorNotSupported
            | Self::QueryFieldForbidden
            | Self::QueryCursorInvalid
            | Self::SecurityContextInvalid => 422,
            Self::TooManyRequests => 429,
            Self::QueryTooComplex | Self::QueryRequiresBatch => 422,
            Self::QueryCursorStale | Self::SecurityPolicyStale => 409,
            Self::QueryProjectionLag => 503,
            Self::QueryTimeout => 504,
            Self::SecurityStepUpRequired => 403,
            Self::SecurityPdpUnavailable | Self::ServiceUnavailable => 503,
            Self::InternalError => 500,
        }
    }

    /// Stable string form used as the wire `code`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::BadRequest => "bad_request",
            Self::Unauthorized => "unauthorized",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Conflict => "conflict",
            Self::PreconditionFailed => "precondition_failed",
            Self::UnprocessableEntity => "unprocessable_entity",
            Self::TooManyRequests => "too_many_requests",
            Self::InternalError => "internal_error",
            Self::ServiceUnavailable => "service_unavailable",
            Self::EntityVersionConflict => "entity.version_conflict",
            Self::IdentityConflict => "identity.conflict",
            Self::IdempotencyConflict => "idempotency.conflict",
            Self::SchemaStateInvalid => "schema.state_invalid",
            Self::SchemaValidationFailed => "schema.validation_failed",
            Self::QuerySyntaxInvalid => "query.syntax_invalid",
            Self::QueryUnknownType => "query.unknown_type",
            Self::QueryUnknownPredicate => "query.unknown_predicate",
            Self::QueryOperatorNotSupported => "query.operator_not_supported",
            Self::QueryFieldForbidden => "query.field_forbidden",
            Self::QueryTooComplex => "query.too_complex",
            Self::QueryRequiresBatch => "query.requires_batch",
            Self::QueryCursorInvalid => "query.cursor_invalid",
            Self::QueryCursorStale => "query.cursor_stale",
            Self::QueryProjectionLag => "query.projection_lag",
            Self::QueryTimeout => "query.timeout",
            Self::SecurityUnauthenticated => "security.unauthenticated",
            Self::SecurityCapabilityDenied => "security.capability_denied",
            Self::SecurityDataDenied => "security.data_denied",
            Self::SecurityStepUpRequired => "security.step_up_required",
            Self::SecurityPolicyStale => "security.policy_stale",
            Self::SecurityPdpUnavailable => "security.pdp_unavailable",
            Self::SecurityContextInvalid => "security.context_invalid",
        }
    }
}

/// API error body (docs/06 §2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Map<String, serde_json::Value>>,
    pub request_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ApiError {
    /// Build an error from a normative code and request id.
    pub fn new(code: ApiErrorCode, request_id: impl Into<String>) -> Self {
        let key = format!("errors.{}", code.as_str());
        Self {
            code: code.as_str().to_string(),
            message_key: key,
            parameters: None,
            request_id: request_id.into(),
            message: None,
        }
    }
}
