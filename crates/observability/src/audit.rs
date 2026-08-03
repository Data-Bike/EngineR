//! Audit event (docs/08 §11, docs/21 §2). Audit is distinct from operational
//! logs: it is a tamper-evident business/security proof with retention.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// High-level action being audited.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    Authentication,
    Decision,
    DeniedRequest,
    PolicyChange,
    PolicySimulation,
    Export,
    BlobRead,
    BulkAction,
    BreakGlass,
    SchemaPublish,
    Mutation,
}

/// Outcome of an audited action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditOutcome {
    Allow,
    Deny,
    Indeterminate,
    Success,
    Failure,
}

/// An audit event. Append-only, time-partitioned, integrity-chained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: uuid::Uuid,
    pub time: DateTime<Utc>,
    pub tenant_id: uuid::Uuid,
    pub actor: uuid::Uuid,
    pub effective_actor: Option<uuid::Uuid>,
    pub session_id: Option<uuid::Uuid>,
    pub service: Option<String>,
    pub action: AuditAction,
    pub resource: String,
    pub outcome: AuditOutcome,
    pub decision_id: Option<uuid::Uuid>,
    pub policy_revision: Option<i64>,
    pub request_id: uuid::Uuid,
    pub correlation_id: Option<uuid::Uuid>,
    pub row_count: Option<i64>,
    pub byte_count: Option<i64>,
    pub reason: Option<String>,
    /// Hash of the previous event (tamper-evident chain).
    pub previous_hash: Option<[u8; 32]>,
    /// Hash of this event's canonical content.
    pub event_hash: [u8; 32],
}
