//! Policy rule model (docs/08 §3).

use enginer_domain::{GroupId, PolicyId, TenantId};
use serde::{Deserialize, Serialize};

/// Effect of a policy rule. Deny overrides allow (ADR-011).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyRuleEffect {
    Allow,
    Deny,
}

/// Resource selector: what the rule applies to (type / entity / predicate /
/// relation / classification / etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSelector {
    /// Type id the rule applies to, or `None` for all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<uuid::Uuid>,
    /// Entity id, or `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<i64>,
    /// Predicate ids (fields/relations) the rule scopes.
    #[serde(default)]
    pub predicate_ids: Vec<uuid::Uuid>,
    /// Classification tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<String>,
    /// Business unit scope handle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub business_unit_scope: Option<String>,
}

/// A policy rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRule {
    pub policy_id: PolicyId,
    pub tenant_id: TenantId,
    pub effect: PolicyRuleEffect,
    /// Subject group the rule applies to.
    pub group_id: GroupId,
    /// Action (capability string) for system rules, or `data.*` for data rules.
    pub action: String,
    pub resource: ResourceSelector,
    /// Optional row condition (security AST reference, never raw SQL).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_condition: Option<String>,
    /// Field mask to apply.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_mask: Option<String>,
    /// Priority (higher wins on conflict; deny still overrides allow).
    pub priority: i32,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
    pub policy_revision: i64,
    pub reason: Option<String>,
}
