//! Request context (docs/05 §7). Created by the Rocket guard and passed
//! explicitly; no thread-local / global mutable state.

use enginer_domain::{
    CorrelationId, IdempotencyKey, LocaleTag, PrincipalId, RequestId, SessionId, TenantId,
    TimeZoneId,
};
use serde::{Deserialize, Serialize};

/// Authentication strength (docs/08 §9, docs/09 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthStrength {
    /// Single factor (password / OIDC code).
    Single,
    /// MFA / step-up performed.
    Mfa,
    /// Break-glass flow (docs/08 §13).
    BreakGlass,
}

/// Purpose of use (docs/08 §2.2, docs/09 §3). Drives data permission scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PurposeOfUse(pub String);

/// The context bound to a single request. Immutable after construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestContext {
    pub tenant_id: TenantId,
    pub principal_id: PrincipalId,
    pub session_id: SessionId,
    pub auth_strength: AuthStrength,
    pub request_id: RequestId,
    pub correlation_id: CorrelationId,
    pub idempotency_key: Option<IdempotencyKey>,
    pub locale: LocaleTag,
    pub time_zone: TimeZoneId,
    pub purpose: PurposeOfUse,
    /// Policy snapshot revision to bind cached decisions/cursors.
    pub policy_revision: Option<i64>,
    /// Schema revision the client expects.
    pub schema_revision: Option<i64>,
    /// Request deadline (UTC).
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
}

impl RequestContext {
    /// Build a system/internal context (e.g. for worker bootstrapping).
    pub fn system(tenant: TenantId, principal: PrincipalId) -> Self {
        Self {
            tenant_id: tenant,
            principal_id: principal,
            session_id: SessionId::new(),
            auth_strength: AuthStrength::Single,
            request_id: RequestId::new(),
            correlation_id: CorrelationId::new(),
            idempotency_key: None,
            locale: LocaleTag::en_us(),
            time_zone: TimeZoneId::utc(),
            purpose: PurposeOfUse("system".to_string()),
            policy_revision: None,
            schema_revision: None,
            deadline: None,
        }
    }
}
