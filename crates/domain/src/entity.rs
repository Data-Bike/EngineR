//! Entity aggregate root (docs/03 §5, docs/04 §10-11).
//!
//! An entity is a node in the attributed multigraph. It has a stable public
//! UUID, an internal compact bigint id, a class, a monotonic revision, and a
//! set of statements (fields + edges). The aggregate enforces:
//!
//! - revision monotonicity (each command bumps revision);
//! - no self-merge (`canonical_entity_id != entity_id`);
//! - retired entities reject further mutations except restore;
//! - merge moves loser public ids to `entity_alias` and points
//!   `canonical_entity_id` at the winner (idempotent on repeat).

use crate::ids::{EntityId, PublicEntityId, TenantId, TypeId};
use crate::schema::EntityClass;
use crate::statement::Revision;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Entity lifecycle status (docs/10 §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityStatus {
    Active,
    Retired,
}

/// The entity aggregate root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub tenant_id: TenantId,
    pub entity_id: EntityId,
    pub public_id: PublicEntityId,
    pub type_id: TypeId,
    pub entity_class: EntityClass,
    pub revision: Revision,
    pub status: EntityStatus,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<uuid::Uuid>,
    pub retired_at: Option<DateTime<Utc>>,
    pub retired_by: Option<uuid::Uuid>,
    pub retire_reason: Option<String>,
    /// After merge: points at the canonical surviving entity.
    pub canonical_entity_id: Option<EntityId>,
    pub classification_id: Option<uuid::Uuid>,
    pub business_unit_id: Option<uuid::Uuid>,
}

impl Entity {
    /// Construct a new active entity.
    #[must_use]
    pub fn new(
        tenant_id: TenantId,
        entity_id: EntityId,
        public_id: PublicEntityId,
        type_id: TypeId,
        entity_class: EntityClass,
        created_by: Option<uuid::Uuid>,
    ) -> Self {
        Self {
            tenant_id,
            entity_id,
            public_id,
            type_id,
            entity_class,
            revision: Revision(1),
            status: EntityStatus::Active,
            created_at: Utc::now(),
            created_by,
            retired_at: None,
            retired_by: None,
            retire_reason: None,
            canonical_entity_id: None,
            classification_id: None,
            business_unit_id: None,
        }
    }

    /// Bump the revision after a successful command.
    pub fn bump_revision(&mut self) {
        self.revision = Revision(self.revision.0 + 1);
    }

    /// True if the entity is retired.
    #[must_use]
    pub fn is_retired(&self) -> bool {
        self.status == EntityStatus::Retired
    }

    /// Retire the entity (idempotent if already retired with same reason).
    ///
    /// # Errors
    ///
    /// Reserved for lifecycle invariant failures introduced by stricter policies.
    pub fn retire(
        &mut self,
        by: Option<uuid::Uuid>,
        reason: String,
        now: DateTime<Utc>,
    ) -> Result<(), crate::DomainError> {
        if self.is_retired() {
            // idempotent: same outcome, no revision bump
            return Ok(());
        }
        self.status = EntityStatus::Retired;
        self.retired_at = Some(now);
        self.retired_by = by;
        self.retire_reason = Some(reason);
        self.bump_revision();
        Ok(())
    }

    /// Restore a retired entity.
    ///
    /// # Errors
    ///
    /// Reserved for lifecycle invariant failures introduced by stricter policies.
    pub fn restore(&mut self, now: DateTime<Utc>) -> Result<(), crate::DomainError> {
        if !self.is_retired() {
            return Ok(()); // idempotent
        }
        self.status = EntityStatus::Active;
        self.retired_at = None;
        self.retired_by = None;
        self.retire_reason = None;
        self.bump_revision();
        let _ = now;
        Ok(())
    }

    /// Mark this entity as merged into `canonical`. Idempotent on repeat.
    ///
    /// # Errors
    ///
    /// Returns an invariant error when an entity is merged into itself.
    pub fn mark_merged_into(&mut self, canonical: EntityId) -> Result<(), crate::DomainError> {
        if canonical == self.entity_id {
            return Err(crate::DomainError::invariant(
                "cannot merge entity into itself",
            ));
        }
        if self.canonical_entity_id == Some(canonical) {
            return Ok(()); // idempotent repeat merge
        }
        self.canonical_entity_id = Some(canonical);
        self.bump_revision();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Entity {
        Entity::new(
            TenantId::new(),
            EntityId::new(),
            PublicEntityId::new(),
            TypeId::new(),
            EntityClass::Data,
            None,
        )
    }

    #[test]
    fn retire_is_idempotent() {
        let mut e = sample();
        let r0 = e.revision;
        e.retire(None, "dup".to_string(), Utc::now()).unwrap();
        let r1 = e.revision;
        assert!(r1.0 > r0.0);
        e.retire(None, "dup".to_string(), Utc::now()).unwrap();
        assert_eq!(e.revision, r1, "second retire must not bump revision");
    }

    #[test]
    fn restore_is_idempotent() {
        let mut e = sample();
        e.retire(None, "x".to_string(), Utc::now()).unwrap();
        let r1 = e.revision;
        e.restore(Utc::now()).unwrap();
        let r2 = e.revision;
        assert!(r2.0 > r1.0);
        e.restore(Utc::now()).unwrap();
        assert_eq!(e.revision, r2);
    }

    #[test]
    fn merge_self_rejected() {
        let mut e = sample();
        let res = e.mark_merged_into(e.entity_id);
        assert!(res.is_err());
    }

    #[test]
    fn merge_idempotent() {
        let mut e = sample();
        let other = EntityId::new();
        e.mark_merged_into(other).unwrap();
        let r1 = e.revision;
        e.mark_merged_into(other).unwrap();
        assert_eq!(e.revision, r1, "repeat merge must not bump revision");
        assert_eq!(e.canonical_entity_id, Some(other));
    }

    #[test]
    fn new_entity_starts_at_revision_1_and_active() {
        let e = sample();
        assert_eq!(e.revision.0, 1);
        assert_eq!(e.status, EntityStatus::Active);
        assert!(!e.is_retired());
        assert!(e.canonical_entity_id.is_none());
        assert!(e.retired_at.is_none());
    }

    #[test]
    fn bump_revision_increments_monotonically() {
        let mut e = sample();
        assert_eq!(e.revision.0, 1);
        e.bump_revision();
        assert_eq!(e.revision.0, 2);
        e.bump_revision();
        assert_eq!(e.revision.0, 3);
    }

    #[test]
    fn retire_then_restore_then_retire_cycle() {
        let mut e = sample();
        e.retire(None, "r1".to_string(), Utc::now()).unwrap();
        assert!(e.is_retired());
        e.restore(Utc::now()).unwrap();
        assert!(!e.is_retired());
        e.retire(None, "r2".to_string(), Utc::now()).unwrap();
        assert!(e.is_retired());
        assert_eq!(e.retire_reason, Some("r2".to_string()));
    }

    #[test]
    fn merge_into_different_canonical_replaces_previous() {
        let mut e = sample();
        let first = EntityId::new();
        let second = EntityId::new();
        e.mark_merged_into(first).unwrap();
        assert_eq!(e.canonical_entity_id, Some(first));
        let r1 = e.revision;
        e.mark_merged_into(second).unwrap();
        assert_eq!(e.canonical_entity_id, Some(second));
        assert!(
            e.revision.0 > r1.0,
            "merging into different canonical must bump"
        );
    }
}
