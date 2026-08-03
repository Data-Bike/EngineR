//! Identity policy and identity keys (docs/04 §6, docs/04 §10).
//!
//! An `IdentityPolicy` defines how an entity of a given type is deduplicated:
//! the scope, key components (predicate ids), normalizer, collision action and
//! resurrection policy. An `IdentityKey` is the computed normalized key for a
//! concrete entity. Concurrent creates with the same key resolve to one entity
//! via a unique index + `ON CONFLICT` (docs/04 §10).

use crate::ids::{EntityId, PredicateId, TenantId, TypeId};
use serde::{Deserialize, Serialize};

/// Collision action when two entities share an identity key
/// (docs/04 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionAction {
    /// Reject the second create with 409.
    Reject,
    /// Return the existing entity (deduplicate).
    ReturnExisting,
    /// Create a `same_as_candidate` link for manual review.
    CreateCandidate,
}

/// Match mode for identity (docs/04 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityMode {
    Exact,
    Fuzzy,
}

/// Resurrection policy for retired entities sharing an identity key
/// (docs/04 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResurrectionPolicy {
    /// Reuse the retired entity (restore it).
    Reuse,
    /// Create a new entity; keep the retired one as an alias.
    NewWithAlias,
    /// Reject; require manual un-retire.
    Reject,
}

/// An identity policy attached to a type (docs/04 §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityPolicy {
    pub policy_id: uuid::Uuid,
    pub tenant_id: TenantId,
    pub type_id: TypeId,
    pub scope: String,
    /// Predicate ids whose normalized values form the key, in order.
    pub key_component_predicate_ids: Vec<PredicateId>,
    pub normalizer_version: String,
    pub match_mode: IdentityMode,
    pub collision_action: CollisionAction,
    pub resurrection_policy: ResurrectionPolicy,
}

/// A computed identity key for a concrete entity. Stored normalized; the raw
/// values are kept on the statements themselves (docs/04 §7).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IdentityKey {
    pub tenant_id: TenantId,
    pub type_id: TypeId,
    pub policy_id: uuid::Uuid,
    /// Canonical key string (normalized components joined by `\u{1f}`).
    pub key: String,
    pub entity_id: EntityId,
    pub status: IdentityKeyStatus,
}

/// Status of an identity key (whether the owning entity is active/retired).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityKeyStatus {
    Active,
    Retired,
}

/// Compute the canonical key string from normalized component values, in
/// policy order. Components are joined by the unit separator `\u{1f}` so that
/// `(a, b)` and `(ab, )` never collide.
pub fn compute_identity_key(components: &[String]) -> String {
    components.join("\u{1f}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_separator_prevents_collision() {
        let a = compute_identity_key(&["ab".to_string(), "".to_string()]);
        let b = compute_identity_key(&["a".to_string(), "b".to_string()]);
        assert_ne!(a, b);
    }

    #[test]
    fn key_order_matters() {
        let a = compute_identity_key(&["x".to_string(), "y".to_string()]);
        let b = compute_identity_key(&["y".to_string(), "x".to_string()]);
        assert_ne!(a, b);
    }
}
