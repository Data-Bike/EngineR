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
///
/// Wire/JSONB shape:
/// - `{"object_kind":"entity","entity_id":<i64>}`
/// - `{"object_kind":"scalar","kind":"<scalar_kind>",...}`
///
/// Implemented manually: serde's internally-tagged representation cannot
/// serialize a newtype variant containing a bare integer.
#[derive(Debug, Clone, PartialEq)]
pub enum StatementObject {
    Entity(crate::ids::EntityId),
    Scalar(CanonicalScalar),
}

impl Serialize for StatementObject {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::Entity(id) => {
                let mut map = serializer.serialize_map(Some(2))?;
                map.serialize_entry("object_kind", "entity")?;
                map.serialize_entry("entity_id", &id.raw())?;
                map.end()
            }
            Self::Scalar(s) => {
                let mut map = serde_json::Map::new();
                map.insert("object_kind".to_string(), serde_json::Value::from("scalar"));
                if let serde_json::Value::Object(inner) =
                    serde_json::to_value(s).map_err(serde::ser::Error::custom)?
                {
                    map.extend(inner);
                }
                map.serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for StatementObject {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let kind = value
            .get("object_kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        match kind {
            "entity" => {
                let id = value
                    .get("entity_id")
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| serde::de::Error::custom("entity object requires entity_id"))?;
                Ok(Self::Entity(crate::ids::EntityId(id)))
            }
            "scalar" => {
                let mut inner = value;
                if let Some(map) = inner.as_object_mut() {
                    map.remove("object_kind");
                }
                serde_json::from_value(inner)
                    .map(Self::Scalar)
                    .map_err(serde::de::Error::custom)
            }
            other => Err(serde::de::Error::custom(format!(
                "unknown object_kind {other:?}"
            ))),
        }
    }
}

impl StatementObject {
    /// True if this is an entity (edge) object.
    #[must_use]
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
    #[must_use]
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

#[cfg(test)]
mod serde_shape_tests {
    use super::*;

    #[test]
    fn object_wire_shape() {
        let e = StatementObject::Entity(crate::ids::EntityId(3));
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"object_kind":"entity","entity_id":3}"#
        );
        let s = StatementObject::Scalar(crate::scalar::CanonicalScalar::Text {
            value_nfc: "x".into(),
            language_tag: None,
        });
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["object_kind"], "scalar");
        assert_eq!(json["kind"], "text");
        let back: StatementObject = serde_json::from_value(json).unwrap();
        assert_eq!(back, s);
        let back_e: StatementObject =
            serde_json::from_str(r#"{"object_kind":"entity","entity_id":3}"#).unwrap();
        assert_eq!(back_e, e);
    }
}
