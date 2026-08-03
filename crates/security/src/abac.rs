//! ABAC: attribute-based access control, policy rules (docs/08 §3).
//!
//! Rule contains: effect allow/deny, subject/group, action, resource
//! selector, condition, fields/relations, mask/obligation, priority,
//! valid interval, policy revision, reason/legal basis (docs/08 §3).
//!
//! Default deny. Explicit deny overrides allow (docs/08 §3).

use serde::{Deserialize, Serialize};

/// Policy effect (docs/08 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    /// Allow access.
    Allow,
    /// Deny access (overrides allow, docs/08 §3).
    Deny,
}

/// Attribute value for ABAC evaluation (docs/08 §2.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AttributeValue {
    /// Tenant ID.
    Tenant { value: String },
    /// Namespace.
    Namespace { value: String },
    /// Entity type alias.
    EntityType { value: String },
    /// Entity public ID.
    EntityId { value: String },
    /// Predicate field name.
    PredicateField { value: String },
    /// Relation predicate.
    RelationPredicate { value: String },
    /// Relation direction.
    RelationDirection { value: String },
    /// Classification level.
    Classification { value: String },
    /// Tag.
    Tag { value: String },
    /// Ownership.
    Owner { value: String },
    /// Row condition (SQL-like expression).
    RowCondition { value: String },
    /// Historical period start.
    PeriodStart { value: String },
    /// Historical period end.
    PeriodEnd { value: String },
    /// Purpose of use.
    Purpose { value: String },
}

/// Resource selector (docs/08 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSelector {
    /// Tenant ID (if specific).
    pub tenant_id: Option<String>,
    /// Entity type alias (if specific).
    pub entity_type: Option<String>,
    /// Entity public ID (if specific).
    pub entity_id: Option<String>,
    /// Predicate fields (empty = all).
    pub fields: Vec<String>,
    /// Relation predicates (empty = all).
    pub relations: Vec<String>,
    /// Classification levels allowed.
    pub classifications: Vec<String>,
    /// Tags required.
    pub tags: Vec<String>,
    /// Row condition (SQL-like, optional).
    pub row_condition: Option<String>,
}

impl Default for ResourceSelector {
    fn default() -> Self {
        Self {
            tenant_id: None,
            entity_type: None,
            entity_id: None,
            fields: Vec::new(),
            relations: Vec::new(),
            classifications: Vec::new(),
            tags: Vec::new(),
            row_condition: None,
        }
    }
}

/// A data permission rule (docs/08 §2.2, §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataRule {
    /// Rule ID.
    pub rule_id: String,
    /// Effect (allow/deny).
    pub effect: Effect,
    /// Subject: user or group ID.
    pub subject: String,
    /// Action (e.g. "read", "write", "export").
    pub action: String,
    /// Resource selector.
    pub resource: ResourceSelector,
    /// Condition expression (optional, DSL).
    pub condition: Option<String>,
    /// Fields allowed/denied.
    pub fields: Vec<String>,
    /// Relations allowed/denied.
    pub relations: Vec<String>,
    /// Mask/obligation (docs/08 §7).
    pub mask: Option<String>,
    /// Obligations (e.g. "log", "watermark").
    pub obligations: Vec<String>,
    /// Priority (higher = evaluated first).
    pub priority: i32,
    /// Valid from.
    pub valid_from: chrono::DateTime<chrono::Utc>,
    /// Valid to (None = indefinite).
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
    /// Policy revision ID.
    pub policy_revision_id: String,
    /// Reason/legal basis.
    pub reason: String,
}

impl DataRule {
    /// Check if the rule is currently valid.
    pub fn is_valid_at(&self, at: chrono::DateTime<chrono::Utc>) -> bool {
        at >= self.valid_from && self.valid_to.map(|v| at <= v).unwrap_or(true)
    }

    /// Check if the rule is a deny rule.
    pub fn is_deny(&self) -> bool {
        self.effect == Effect::Deny
    }

    /// Check if the rule matches a subject and action.
    pub fn matches(&self, subject: &str, action: &str) -> bool {
        self.subject == subject && self.action == action
    }
}

/// A policy rule (combines data rules, docs/08 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule ID.
    pub rule_id: String,
    /// Effect.
    pub effect: Effect,
    /// Subject (user or group ID).
    pub subject: String,
    /// Action.
    pub action: String,
    /// Resource selector.
    pub resource: ResourceSelector,
    /// Condition (optional).
    pub condition: Option<String>,
    /// Priority.
    pub priority: i32,
    /// Valid from.
    pub valid_from: chrono::DateTime<chrono::Utc>,
    /// Valid to.
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
    /// Policy revision ID.
    pub policy_revision_id: String,
    /// Reason/legal basis.
    pub reason: String,
}

/// Data permission (docs/08 §2.2).
///
/// Represents the resolved data access permissions for a principal,
/// combining all applicable data rules.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPermission {
    /// Allowed entity types.
    pub allowed_types: Vec<String>,
    /// Denied entity types.
    pub denied_types: Vec<String>,
    /// Allowed fields per type.
    pub allowed_fields: Vec<(String, Vec<String>)>,
    /// Denied fields per type.
    pub denied_fields: Vec<(String, Vec<String>)>,
    /// Allowed relations.
    pub allowed_relations: Vec<String>,
    /// Denied relations.
    pub denied_relations: Vec<String>,
    /// Row condition (SQL-like, optional).
    pub row_condition: Option<String>,
    /// Allowed classifications.
    pub allowed_classifications: Vec<String>,
    /// Obligations (e.g. "log", "watermark").
    pub obligations: Vec<String>,
    /// Mask strategy (if any).
    pub mask: Option<String>,
}

impl Default for DataPermission {
    fn default() -> Self {
        Self {
            allowed_types: Vec::new(),
            denied_types: Vec::new(),
            allowed_fields: Vec::new(),
            denied_fields: Vec::new(),
            allowed_relations: Vec::new(),
            denied_relations: Vec::new(),
            row_condition: None,
            allowed_classifications: Vec::new(),
            obligations: Vec::new(),
            mask: None,
        }
    }
}

impl DataPermission {
    /// Check if an entity type is allowed.
    pub fn is_type_allowed(&self, entity_type: &str) -> bool {
        !self.denied_types.contains(&entity_type.to_string())
            && (self.allowed_types.is_empty()
                || self.allowed_types.contains(&entity_type.to_string()))
    }

    /// Check if a field is allowed for a type.
    pub fn is_field_allowed(&self, entity_type: &str, field: &str) -> bool {
        let denied = self
            .denied_fields
            .iter()
            .find(|(t, _)| t == entity_type)
            .map(|(_, f)| f.contains(&field.to_string()))
            .unwrap_or(false);
        if denied {
            return false;
        }
        self.allowed_fields
            .iter()
            .find(|(t, _)| t == entity_type)
            .map(|(_, f)| f.is_empty() || f.contains(&field.to_string()))
            .unwrap_or(true)
    }

    /// Check if a relation is allowed.
    pub fn is_relation_allowed(&self, relation: &str) -> bool {
        !self.denied_relations.contains(&relation.to_string())
            && (self.allowed_relations.is_empty()
                || self.allowed_relations.contains(&relation.to_string()))
    }
}
