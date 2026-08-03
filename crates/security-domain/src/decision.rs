//! Security decisions and obligations (docs/09 §4-8).
//!
//! Decisions are bound to tenant/principal/session/auth strength/action/
//! resource/query fingerprint/schema+policy revisions/purpose/expiry
//! (docs/09 §9). A `DecisionId` is **not** a bearer permission.

use enginer_domain::{DecisionId, PolicyId};
use serde::{Deserialize, Serialize};

/// Effect of a decision (docs/09 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionEffect {
    Allow,
    AllowWithObligations,
    Deny,
    /// Indeterminate is treated as deny by the PEP (fail closed).
    Indeterminate,
    /// Step-up authentication required.
    Challenge,
}

/// Field-level policy (docs/08 §7, docs/09 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldPolicy {
    /// Key omitted entirely (preferred).
    Omit,
    /// Key present, value null.
    Null,
    /// Fixed replacement value.
    Fixed,
    /// Partial mask (prefix/suffix).
    Partial,
    /// Hash/token replacement.
    HashToken,
    /// Bucket/range replacement.
    Bucket,
    /// Aggregate-only (no individual value).
    AggregateOnly,
}

/// A field mask applied to a predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldMask {
    pub predicate_id: uuid::Uuid,
    pub policy: FieldPolicy,
    /// Algorithm id + version for the chosen mask.
    pub algorithm: Option<String>,
}

/// A mandatory row filter expressed as a security AST reference (never raw SQL,
/// docs/09 §7). The query engine compiles it together with the user AST.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MandatoryFilter {
    /// Reference to a pre-registered system field/scope expression.
    pub system_field: String,
    pub op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
    pub scope: Option<String>,
}

/// Obligations attached to a query decision (docs/09 §5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryObligations {
    pub allowed_root_type_ids: Vec<uuid::Uuid>,
    pub denied_type_ids: Vec<uuid::Uuid>,
    pub allowed_predicate_ids: Vec<uuid::Uuid>,
    pub denied_predicate_ids: Vec<uuid::Uuid>,
    pub allowed_relations: Vec<AllowedRelation>,
    pub mandatory_filters: Vec<MandatoryFilter>,
    pub masks: Vec<FieldMask>,
    pub max_page_size: u32,
    pub max_graph_depth: u32,
    pub max_graph_nodes: u32,
    /// `"allow"`, `"deny"` or `"require_min_group_size"`.
    pub aggregates: String,
}

/// An allowed relation direction/target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AllowedRelation {
    pub predicate_id: uuid::Uuid,
    pub directions: Vec<String>,
    pub target_type_ids: Vec<uuid::Uuid>,
}

/// Obligations attached to a mutation decision (docs/09 §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationObligations {
    pub allowed_changed_predicate_ids: Vec<uuid::Uuid>,
    pub forbidden_relation_targets: Vec<uuid::Uuid>,
    pub max_batch: u32,
    pub required_reason: bool,
    pub audit_level: String,
}

/// Obligations attached to an action decision (docs/09 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataObligations {
    pub max_rows: Option<u64>,
    pub watermark: Option<String>,
    pub require_step_up: bool,
    pub audit_level: String,
}

/// The full obligations union.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Obligations {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<DataObligations>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<QueryObligations>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mutation: Option<MutationObligations>,
}

/// A security decision (docs/09 §4-6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecurityDecision {
    pub decision_id: DecisionId,
    pub effect: DecisionEffect,
    pub policy_revision: i64,
    pub obligations: Obligations,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub policy_ids: Vec<PolicyId>,
}
