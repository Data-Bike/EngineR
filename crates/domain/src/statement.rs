//! Statement — the single logical atom (docs/03 §3, docs/04 §2).
//!
//! A statement is `subject — predicate → object`, where object is either an
//! entity (edge) or a canonical scalar (property). This is the unitary
//! constructor: properties and links differ only in the kind of object.

use crate::fingerprint::FactFingerprint;
use crate::ids::{PredicateId, StatementId, TenantId};
use crate::provenance::ProvenanceRef;
use crate::scalar::CanonicalScalar;
use crate::time::{RecordedTime, ValidPeriod};
use serde::{Deserialize, Serialize};

/// The object side of a statement: exactly one of entity or scalar
/// (invariant 1, docs/03 §12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "object_kind", rename_all = "snake_case")]
pub enum StatementObject {
    Entity(crate::ids::EntityId),
    Scalar(CanonicalScalar),
}

impl StatementObject {
    /// True if this is an entity (edge) object.
    pub fn is_edge(&self) -> bool {
        matches!(self, Self::Entity(_))
    }
}

/// Statement status (docs/10 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StatementStatus {
    Active,
    Superseded,
    Revoked,
    Quarantined,
}

/// Confidence bucket (0.0–1.0 stored as canonical string in persistence).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(pub f32);

impl Confidence {
    /// Maximum confidence.
    pub const fn max() -> Self {
        Self(1.0)
    }
}

/// Entity revision counter (monotonic per entity).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Revision(pub i64);

/// Valid time wrapper (business truth). Distinct from recorded time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidTime(pub ValidPeriod);

/// A statement (logical atom).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Statement {
    pub id: StatementId,
    pub tenant_id: TenantId,
    pub subject: crate::ids::EntityId,
    pub predicate: PredicateId,
    pub object: StatementObject,
    pub validity: ValidTime,
    pub recorded: RecordedTime,
    pub provenance: ProvenanceRef,
    pub confidence: Confidence,
    pub status: StatementStatus,
    pub revision: Revision,
    /// Fact fingerprint (BLAKE2b-256) for dedupe acceleration.
    pub fingerprint: FactFingerprint,
    /// Statement this one supersedes (immutable history, ADR-004).
    pub supersedes_statement_id: Option<StatementId>,
    /// Statement that revoked this one.
    pub revoked_by_statement_id: Option<StatementId>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::EntityId;
    use crate::scalar::CanonicalScalar;
    use chrono::Utc;

    #[test]
    fn object_kind_discriminant() {
        let s = StatementObject::Scalar(CanonicalScalar::Boolean { value: true });
        assert!(!s.is_edge());
        let e = StatementObject::Entity(EntityId::new());
        assert!(e.is_edge());
    }

    #[test]
    fn valid_time_wraps_period() {
        let vt = ValidTime(crate::time::ValidPeriod::open_from(Utc::now()));
        assert!(vt.0.valid_to.is_none());
    }
}
