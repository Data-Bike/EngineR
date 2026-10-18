//! Schema registry domain types (docs/03 §4-6, docs/12).
//!
//! `TypeDefinition` and `PredicateDefinition` are stable-ID entities of the
//! meta-kernel. Renaming a label does not migrate facts/policies/views
//! (ADR-003). Published revisions are immutable (docs/12 §1).

use crate::ids::{PredicateId, SchemaRevisionId, TenantId, TypeId};
use serde::{Deserialize, Serialize};

/// Entity class. Drives physical storage and security defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityClass {
    /// Business data entity (person, `legal_entity`, item, ...).
    Data,
    /// Schema-meta entity (type, predicate, view definition).
    Schema,
    /// Security entity (principal, group, policy).
    Policy,
    /// Workflow entity (process, task, timer).
    Workflow,
    /// System entity (tenant, connector, projection catalog).
    System,
}

/// Direction of a predicate (docs/03 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PredicateDirection {
    /// Directed edge: subject → object.
    Directed,
    /// Symmetric edge: endpoints sorted canonically (docs/04 §7).
    Symmetric,
}

/// Lifecycle state of a schema revision (docs/12 §1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaRevisionState {
    Draft,
    Validating,
    ImpactReady,
    Approved,
    Publishing,
    Published,
    Retired,
    Failed,
}

/// A predicate definition (property or link). Stable `PredicateId`; the ASCII
/// alias is ergonomic only and may change without migrating facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredicateDefinition {
    pub predicate_id: PredicateId,
    /// Ergonomic ASCII alias. Not a DB/API key.
    pub stable_alias: String,
    /// True for entity-object (edge), false for scalar.
    pub is_edge: bool,
    pub direction: PredicateDirection,
    /// Domain type id (subject side).
    pub domain_type_id: Option<TypeId>,
    /// Range type id (object side for edges) or scalar kind.
    pub range_type_id: Option<TypeId>,
    pub scalar_kind: Option<crate::scalar::ScalarKind>,
    /// Cardinality bounds.
    pub min_cardinality: u32,
    pub max_cardinality: Option<u32>,
    /// Whether this predicate participates in identity keys.
    pub participates_in_identity: bool,
    /// Confidentiality class.
    pub confidentiality_class: String,
    /// Normalizer version reference (for scalar predicates).
    pub normalizer_version: Option<String>,
}

/// A type definition (docs/03 §5). A type is a named set of constraints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub type_id: TypeId,
    /// Ergonomic ASCII alias.
    pub stable_alias: String,
    pub entity_class: EntityClass,
    /// Required/allowed predicate ids with cardinality.
    pub predicates: Vec<TypePredicateSlot>,
    /// Identity policy ids that apply to this type.
    pub identity_policy_ids: Vec<uuid::Uuid>,
}

/// A predicate slot within a type definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypePredicateSlot {
    pub predicate_id: PredicateId,
    pub required: bool,
    pub min_cardinality: u32,
    pub max_cardinality: Option<u32>,
    pub order: Option<u32>,
    pub group: Option<String>,
}

/// A schema revision (immutable once published).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaRevision {
    pub revision_id: SchemaRevisionId,
    pub tenant_id: TenantId,
    pub state: SchemaRevisionState,
    pub parent_revision_id: Option<SchemaRevisionId>,
    pub semantic_version: String,
    pub content_hash: crate::ids::ContentHash,
    pub types: Vec<TypeDefinition>,
    pub predicates: Vec<PredicateDefinition>,
}
