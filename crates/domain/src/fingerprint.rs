//! Fact fingerprint (docs/04 §8).
//!
//! `tenant | canonical_subject | predicate | object_kind | canonical_object |
//! valid_from | valid_to | qualifier_hash | identity_scope`
//!
//! BLAKE2b-256 accelerates lookup; collision is verified against the full
//! canonical columns/bytes by persistence (never by hash alone).

use crate::ids::{PredicateId, TenantId};
use crate::statement::StatementObject;
use crate::time::ValidPeriod;
use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};

/// A 32-byte fact fingerprint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct FactFingerprint(pub [u8; 32]);

impl FactFingerprint {
    /// Hex encoding.
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

/// Compute a fact fingerprint from the canonical components.
///
/// `qualifier_hash` is an opaque 32-byte hash of out-of-band qualifiers
/// (provenance set id, confidence bucket, etc.) or `None`.
/// `identity_scope` is the identity policy scope tag or `None`.
pub fn fact_fingerprint(
    tenant: TenantId,
    subject: i64,
    predicate: PredicateId,
    object: &StatementObject,
    validity: &ValidPeriod,
    qualifier_hash: Option<&[u8; 32]>,
    identity_scope: Option<&str>,
) -> FactFingerprint {
    let mut h = Blake2b::<U32>::new();
    h.update(tenant.raw().as_bytes());
    h.update(b"|");
    h.update(subject.to_le_bytes());
    h.update(b"|");
    h.update(predicate.raw().as_bytes());
    h.update(b"|");
    match object {
        StatementObject::Entity(eid) => {
            h.update(b"entity:");
            h.update(eid.raw().to_le_bytes());
        }
        StatementObject::Scalar(s) => {
            h.update(b"scalar:");
            h.update(s.canonical_bytes());
        }
    }
    h.update(b"|");
    h.update(validity.valid_from.to_rfc3339().as_bytes());
    h.update(b"|");
    if let Some(to) = validity.valid_to {
        h.update(to.to_rfc3339().as_bytes());
    }
    h.update(b"|");
    if let Some(q) = qualifier_hash {
        h.update(q);
    }
    h.update(b"|");
    if let Some(scope) = identity_scope {
        h.update(scope.as_bytes());
    }
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(out.as_slice());
    FactFingerprint(buf)
}

// Re-export so the public API matches the lib re-export.
pub use crate::statement::ValidTime as _ValidTimeReexport;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{EntityId, PublicEntityId};
    use crate::scalar::CanonicalScalar;
    use crate::statement::ValidTime;
    use chrono::TimeZone;
    use chrono::Utc;

    #[test]
    fn same_facts_same_fingerprint() {
        let tenant = TenantId::new();
        let pred = PredicateId::new();
        let obj = StatementObject::Scalar(CanonicalScalar::Text {
            value_nfc: "ООО Пример".to_string(),
            language_tag: None,
        });
        let vp = ValidPeriod::open_from(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
        let a = fact_fingerprint(tenant, 1, pred, &obj, &vp, None, None);
        let b = fact_fingerprint(tenant, 1, pred, &obj, &vp, None, None);
        assert_eq!(a, b);
        assert_eq!(a.to_hex().len(), 64);
    }

    #[test]
    fn different_subject_different_fingerprint() {
        let tenant = TenantId::new();
        let pred = PredicateId::new();
        let obj = StatementObject::Scalar(CanonicalScalar::Boolean { value: true });
        let vp = ValidPeriod::open_from(Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
        let a = fact_fingerprint(tenant, 1, pred, &obj, &vp, None, None);
        let b = fact_fingerprint(tenant, 2, pred, &obj, &vp, None, None);
        assert_ne!(a, b);
    }

    // suppress unused import warnings for re-exported types in doctests/examples
    #[allow(dead_code)]
    fn _suppress(_a: EntityId, _b: PublicEntityId, _c: ValidTime) {
        let _ = EntityId::new();
        let _ = PublicEntityId::new();
        let _ = ValidTime(crate::time::ValidPeriod::open_from(Utc::now()));
    }
}
