//! Strongly-typed identifiers.
//!
//! Identifiers are never interchangeable `String`s (docs/05 §4). Each ID is a
//! newtype wrapping either a `uuid::Uuid` (public/stable IDs) or a `i64`
//! (internal compact bigint used in physical indexes, docs/10 §3).
//!
//! Public IDs use `UUIDv7` (time-ordered) for API stability; internal bigints are
//! never exposed over the API (docs/06 §1).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Declares a newtype id with `new()`, `raw()`, `Default`, serde transparent,
/// and the common derives (including `Copy`). Does NOT generate
/// `Display`/`FromStr`. Use for `Copy` backing types (`Uuid`, `i64`, arrays).
macro_rules! id_newtype {
    ($(#[$meta:meta])* $name:ident, $backing:ty, $gen:expr) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[repr(transparent)]
        #[serde(transparent)]
        pub struct $name(pub $backing);

        impl $name {
            #[doc = "Generate a new id."]
            pub fn new() -> Self {
                Self($gen)
            }

            #[doc = "Return the raw backing value."]
            pub fn raw(self) -> $backing {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Like `id_newtype!` but without `Copy` (for `String`-backed ids).
macro_rules! id_newtype_nocopy {
    ($(#[$meta:meta])* $name:ident, $backing:ty, $gen:expr) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
        )]
        #[repr(transparent)]
        #[serde(transparent)]
        pub struct $name(pub $backing);

        impl $name {
            #[doc = "Generate a new id."]
            pub fn new() -> Self {
                Self($gen)
            }

            #[doc = "Return a clone of the raw backing value."]
            pub fn raw(&self) -> $backing {
                self.0.clone()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

/// Implements `Display` (`prefix:value`) and `FromStr` for ids whose backing
/// type implements `Display` + `FromStr`.
macro_rules! impl_id_display_fromstr {
    ($name:ident, $backing:ty, $prefix:literal) => {
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }

        impl FromStr for $name {
            type Err = String;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let rest = s.strip_prefix(concat!($prefix, ":")).unwrap_or(s);
                let v: $backing = rest
                    .parse()
                    .map_err(|e: <$backing as FromStr>::Err| format!("{e:?}"))?;
                Ok(Self(v))
            }
        }
    };
}

id_newtype!(
    /// Tenant identifier. Every data entity belongs to exactly one tenant.
    TenantId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(TenantId, Uuid, "tenant");

id_newtype!(
    /// Stable public entity identifier (`UUIDv7`). Exposed over the API.
    PublicEntityId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(PublicEntityId, Uuid, "ent");

id_newtype!(
    /// Internal compact entity identifier (bigint). Never exposed over the API.
    EntityId,
    i64,
    {
        use std::sync::atomic::{AtomicI64, Ordering};
        static COUNTER: AtomicI64 = AtomicI64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
);
impl_id_display_fromstr!(EntityId, i64, "eid");

id_newtype!(
    /// Stable type identifier. Identity is the UUID; the alias is ergonomic only.
    TypeId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(TypeId, Uuid, "type");

id_newtype!(
    /// Stable predicate identifier. Renaming the label does not migrate facts.
    PredicateId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(PredicateId, Uuid, "pred");

id_newtype!(
    /// Schema revision identifier (immutable once published).
    SchemaRevisionId,
    i64,
    {
        use std::sync::atomic::{AtomicI64, Ordering};
        static COUNTER: AtomicI64 = AtomicI64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
);
impl_id_display_fromstr!(SchemaRevisionId, i64, "srev");

id_newtype!(
    /// Statement identifier (logical assertion).
    StatementId,
    i64,
    {
        use std::sync::atomic::{AtomicI64, Ordering};
        static COUNTER: AtomicI64 = AtomicI64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
);
impl_id_display_fromstr!(StatementId, i64, "stmt");

id_newtype!(
    /// Security policy identifier.
    PolicyId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(PolicyId, Uuid, "pol");

id_newtype!(
    /// Security decision identifier. Not a bearer permission (docs/09 §9).
    DecisionId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(DecisionId, Uuid, "dec");

id_newtype!(
    /// Principal identifier (user or service account).
    PrincipalId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(PrincipalId, Uuid, "princ");

id_newtype!(
    /// Group identifier (capability group or data group, docs/08 §2).
    GroupId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(GroupId, Uuid, "grp");

id_newtype!(
    /// Session identifier.
    SessionId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(SessionId, Uuid, "sess");

id_newtype!(
    /// Request identifier (correlates logs/traces/audit for one request).
    RequestId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(RequestId, Uuid, "req");

id_newtype!(
    /// Correlation identifier (cross-request business flow).
    CorrelationId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(CorrelationId, Uuid, "corr");

id_newtype_nocopy!(
    /// Idempotency key (transport idempotency, docs/04 §9).
    IdempotencyKey,
    String,
    Uuid::now_v7().to_string()
);
impl_id_display_fromstr!(IdempotencyKey, String, "idem");

id_newtype!(
    /// Background job identifier.
    JobId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(JobId, Uuid, "job");

id_newtype!(
    /// Outbox/inbox event identifier.
    EventId,
    Uuid,
    Uuid::now_v7()
);
impl_id_display_fromstr!(EventId, Uuid, "evt");

id_newtype!(
    /// Projection generation / revision watermark.
    ProjectionRevision,
    i64,
    {
        use std::sync::atomic::{AtomicI64, Ordering};
        static COUNTER: AtomicI64 = AtomicI64::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
);
impl_id_display_fromstr!(ProjectionRevision, i64, "proj");

id_newtype_nocopy!(
    /// BCP-47 locale tag wrapper (e.g. `ru-RU`, `ar-SA`).
    LocaleTag,
    String,
    "en-US".to_string()
);
impl_id_display_fromstr!(LocaleTag, String, "loc");

impl LocaleTag {
    /// The default/fallback locale.
    #[must_use]
    pub fn en_us() -> Self {
        Self("en-US".to_string())
    }
}

id_newtype_nocopy!(
    /// IANA time zone identifier wrapper (e.g. `Europe/Moscow`).
    TimeZoneId,
    String,
    "UTC".to_string()
);
impl_id_display_fromstr!(TimeZoneId, String, "tz");

impl TimeZoneId {
    /// UTC time zone.
    #[must_use]
    pub fn utc() -> Self {
        Self("UTC".to_string())
    }
}

id_newtype!(
    /// Content hash (BLAKE2b-256) of a blob or canonical value.
    ContentHash,
    [u8; 32],
    [0u8; 32]
);

impl ContentHash {
    /// Hex encoding for transport / logging.
    #[must_use]
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    /// Construct from a 32-byte array.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_roundtrip_display_fromstr() {
        let t = TenantId::new();
        let s = t.to_string();
        let back: TenantId = s.parse().unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn idempotency_key_roundtrip() {
        let k = IdempotencyKey("abc-123".to_string());
        let s = k.to_string();
        assert_eq!(s, "idem:abc-123");
        let back: IdempotencyKey = s.parse().unwrap();
        assert_eq!(k, back);
    }

    #[test]
    fn content_hash_hex() {
        let h = ContentHash([0xab; 32]);
        assert_eq!(h.to_hex().len(), 64);
    }

    #[test]
    fn locale_default() {
        assert_eq!(LocaleTag::new().raw(), "en-US");
        assert_eq!(TimeZoneId::new().raw(), "UTC");
    }
}
