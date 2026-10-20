//! Canonical scalar kinds (docs/03 §7).
//!
//! `f64` is forbidden for money, shares and deterministic scores. Money and
//! measurements use `Decimal` (stored as canonical string + scale over the wire,
//! `numeric(p,s)` in `PostgreSQL`). Instants are UTC + source IANA zone.

use crate::ids::ContentHash;
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Kind of scalar value. Drives physical statement family selection
/// (docs/10 §6) and the query-engine operator allowlist (docs/07 §3).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScalarKind {
    /// Unicode text (NFC normalized).
    Text,
    /// Language-tagged text.
    LocalizedText,
    /// Boolean.
    Boolean,
    /// Arbitrary precision integer.
    Integer,
    /// Arbitrary precision decimal (canonical scale).
    Decimal,
    /// Calendar date (no time, no zone).
    Date,
    /// UTC instant.
    Instant,
    /// Local datetime + IANA source zone.
    LocalDateTime,
    /// UUID / opaque id.
    Uuid,
    /// URI.
    Uri,
    /// E-mail.
    Email,
    /// Phone (E.164 + extension).
    Phone,
    /// Money: decimal + ISO 4217 currency.
    Money,
    /// Measurement: decimal + unit.
    Measurement,
    /// Geo point / shape (`PostGIS` when profile enabled).
    Geo,
    /// Structured JSON only by explicit predicate schema.
    Json,
    /// Immutable blob reference.
    Blob,
}

/// A canonical scalar value. The raw form is stored separately by persistence;
/// the domain only reasons about the canonical form (docs/04 §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CanonicalScalar {
    Text {
        value_nfc: String,
        language_tag: Option<String>,
    },
    Boolean {
        value: bool,
    },
    Integer {
        value: i128,
    },
    Decimal {
        /// Canonical decimal string (no trailing zeros, explicit sign).
        value: String,
    },
    Date {
        value: NaiveDate,
    },
    Instant {
        value: DateTime<Utc>,
        source_zone: Option<String>,
    },
    /// Local wall-clock datetime tagged with the IANA zone it was recorded in
    /// (docs/03 §7). Not a UTC instant — the offset may change with DST rules.
    #[serde(rename = "local_datetime")]
    LocalDateTime {
        value: NaiveDateTime,
        zone: String,
    },
    Uuid {
        value: uuid::Uuid,
    },
    Uri {
        value: String,
    },
    Email {
        local: String,
        domain: String,
    },
    Phone {
        e164: String,
        extension: Option<String>,
    },
    Money {
        amount: String,
        currency: String,
    },
    Measurement {
        amount: String,
        unit: String,
    },
    Json {
        /// Canonical JSON text (sorted keys, no insignificant whitespace).
        value: String,
    },
    /// Geo point/shape — canonical GeoJSON text (PostGIS when the profile is
    /// enabled; stored as canonical JSON meanwhile).
    Geo {
        value: String,
    },
    Blob {
        content_hash: ContentHash,
        size_bytes: u64,
        mime: String,
    },
}

impl CanonicalScalar {
    /// The scalar kind of this value.
    #[must_use]
    pub fn kind(&self) -> ScalarKind {
        match self {
            Self::Text {
                language_tag: Some(_),
                ..
            } => ScalarKind::LocalizedText,
            Self::Text { .. } => ScalarKind::Text,
            Self::Boolean { .. } => ScalarKind::Boolean,
            Self::Integer { .. } => ScalarKind::Integer,
            Self::Decimal { .. } => ScalarKind::Decimal,
            Self::Date { .. } => ScalarKind::Date,
            Self::Instant { .. } => ScalarKind::Instant,
            Self::LocalDateTime { .. } => ScalarKind::LocalDateTime,
            Self::Uuid { .. } => ScalarKind::Uuid,
            Self::Uri { .. } => ScalarKind::Uri,
            Self::Email { .. } => ScalarKind::Email,
            Self::Phone { .. } => ScalarKind::Phone,
            Self::Money { .. } => ScalarKind::Money,
            Self::Measurement { .. } => ScalarKind::Measurement,
            Self::Json { .. } => ScalarKind::Json,
            Self::Geo { .. } => ScalarKind::Geo,
            Self::Blob { .. } => ScalarKind::Blob,
        }
    }

    /// Canonical byte representation for fingerprinting. Stable across processes
    /// (deterministic ordering, no locale-dependent formatting).
    #[must_use]
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        let kind = self.kind();
        buf.extend_from_slice(kind.tag().as_bytes());
        buf.push(b'|');
        match self {
            Self::Text {
                value_nfc,
                language_tag,
            } => {
                buf.extend_from_slice(value_nfc.as_bytes());
                if let Some(tag) = language_tag {
                    buf.push(b'@');
                    buf.extend_from_slice(tag.as_bytes());
                }
            }
            Self::Boolean { value } => buf.push(if *value { b'1' } else { b'0' }),
            Self::Integer { value } => buf.extend_from_slice(value.to_string().as_bytes()),
            Self::Decimal { value }
            | Self::Uri { value }
            | Self::Json { value }
            | Self::Geo { value } => {
                buf.extend_from_slice(value.as_bytes());
            }
            Self::Date { value } => buf.extend_from_slice(value.to_string().as_bytes()),
            Self::Instant { value, .. } => buf.extend_from_slice(value.to_rfc3339().as_bytes()),
            Self::LocalDateTime { value, zone } => {
                buf.extend_from_slice(value.to_string().as_bytes());
                buf.push(b'[');
                buf.extend_from_slice(zone.as_bytes());
                buf.push(b']');
            }
            Self::Uuid { value } => buf.extend_from_slice(value.as_bytes()),
            Self::Email { local, domain } => {
                buf.extend_from_slice(local.as_bytes());
                buf.push(b'@');
                buf.extend_from_slice(domain.as_bytes());
            }
            Self::Phone { e164, extension } => {
                buf.extend_from_slice(e164.as_bytes());
                if let Some(ext) = extension {
                    buf.push(b'x');
                    buf.extend_from_slice(ext.as_bytes());
                }
            }
            Self::Money { amount, currency } => {
                buf.extend_from_slice(amount.as_bytes());
                buf.push(b'|');
                buf.extend_from_slice(currency.as_bytes());
            }
            Self::Measurement { amount, unit } => {
                buf.extend_from_slice(amount.as_bytes());
                buf.push(b'|');
                buf.extend_from_slice(unit.as_bytes());
            }
            Self::Blob {
                content_hash,
                size_bytes,
                mime,
            } => {
                buf.extend_from_slice(&content_hash.raw());
                buf.push(b'|');
                buf.extend_from_slice(size_bytes.to_string().as_bytes());
                buf.push(b'|');
                buf.extend_from_slice(mime.as_bytes());
            }
        }
        buf
    }
}

impl ScalarKind {
    fn tag(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::LocalizedText => "ltext",
            Self::Boolean => "bool",
            Self::Integer => "int",
            Self::Decimal => "dec",
            Self::Date => "date",
            Self::Instant => "inst",
            Self::LocalDateTime => "ldt",
            Self::Uuid => "uuid",
            Self::Uri => "uri",
            Self::Email => "email",
            Self::Phone => "phone",
            Self::Money => "money",
            Self::Measurement => "meas",
            Self::Geo => "geo",
            Self::Json => "json",
            Self::Blob => "blob",
        }
    }
}

impl fmt::Display for ScalarKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_bytes_stable() {
        let a = CanonicalScalar::Text {
            value_nfc: "ООО Пример".to_string(),
            language_tag: None,
        };
        let b = CanonicalScalar::Text {
            value_nfc: "ООО Пример".to_string(),
            language_tag: None,
        };
        assert_eq!(a.canonical_bytes(), b.canonical_bytes());
        assert_eq!(a.kind(), ScalarKind::Text);
    }

    #[test]
    fn phone_extension_changes_bytes() {
        let a = CanonicalScalar::Phone {
            e164: "+74950000000".to_string(),
            extension: None,
        };
        let b = CanonicalScalar::Phone {
            e164: "+74950000000".to_string(),
            extension: Some("123".to_string()),
        };
        assert_ne!(a.canonical_bytes(), b.canonical_bytes());
    }

    #[test]
    fn localized_text_reports_localized_kind() {
        let s = CanonicalScalar::Text {
            value_nfc: "Привет".to_string(),
            language_tag: Some("ru".to_string()),
        };
        assert_eq!(s.kind(), ScalarKind::LocalizedText);
        assert!(String::from_utf8_lossy(&s.canonical_bytes()).starts_with("ltext|"));
    }

    #[test]
    fn local_datetime_bytes_carry_zone() {
        let a = CanonicalScalar::LocalDateTime {
            value: NaiveDate::from_ymd_opt(2026, 10, 20)
                .unwrap()
                .and_hms_opt(14, 30, 0)
                .unwrap(),
            zone: "Europe/Moscow".to_string(),
        };
        let b = CanonicalScalar::LocalDateTime {
            value: NaiveDate::from_ymd_opt(2026, 10, 20)
                .unwrap()
                .and_hms_opt(14, 30, 0)
                .unwrap(),
            zone: "UTC".to_string(),
        };
        assert_eq!(a.kind(), ScalarKind::LocalDateTime);
        assert_ne!(a.canonical_bytes(), b.canonical_bytes());
        assert!(String::from_utf8_lossy(&a.canonical_bytes()).contains("[Europe/Moscow]"));
    }

    #[test]
    fn geo_roundtrip_and_kind() {
        let s = CanonicalScalar::Geo {
            value: "{\"type\":\"Point\",\"coordinates\":[37.6,55.7]}".to_string(),
        };
        assert_eq!(s.kind(), ScalarKind::Geo);
        // serde tag stays snake_case for the wire/JSONB form.
        let v = serde_json::to_value(&s).unwrap();
        assert_eq!(v["kind"], "geo");
        let back: CanonicalScalar = serde_json::from_value(v).unwrap();
        assert_eq!(back, s);

        let ldt = CanonicalScalar::LocalDateTime {
            value: NaiveDate::from_ymd_opt(2026, 1, 1)
                .unwrap()
                .and_hms_opt(0, 0, 0)
                .unwrap(),
            zone: "UTC".to_string(),
        };
        let v = serde_json::to_value(&ldt).unwrap();
        assert_eq!(v["kind"], "local_datetime");
        assert_eq!(serde_json::from_value::<CanonicalScalar>(v).unwrap(), ldt);
    }
}
