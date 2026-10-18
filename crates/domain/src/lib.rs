//! Pure domain types, newtypes and invariants for `EngineR`.
//!
//! Normative sources:
//! - `docs/03-universal-domain-model.md`
//! - `docs/04-topology-unitarity-idempotency.md`
//! - `docs/05-rust-workspace.md` §4 (newtypes)
//!
//! This crate has **no** dependency on Rocket, sqlx, Nuxt or any adapter.
//! It only depends on `serde`, `uuid`, `chrono`, `blake2` for canonical
//! serialization / fingerprinting of domain values.

#![forbid(unsafe_code)]

pub mod entity;
pub mod error;
pub mod fingerprint;
pub mod identity;
pub mod ids;
pub mod provenance;
pub mod scalar;
pub mod schema;
pub mod statement;
pub mod time;

pub use entity::{Entity, EntityStatus};
pub use error::DomainError;
pub use fingerprint::fact_fingerprint;
pub use identity::{
    compute_identity_key, CollisionAction, IdentityKey, IdentityKeyStatus, IdentityMode,
    IdentityPolicy, ResurrectionPolicy,
};
pub use ids::*;
pub use provenance::{ProvenanceRef, SourceRef};
pub use scalar::{CanonicalScalar, ScalarKind};
pub use schema::{
    EntityClass, PredicateDefinition, PredicateDirection, SchemaRevision, SchemaRevisionState,
    TypeDefinition,
};
pub use statement::{Confidence, Revision, Statement, StatementObject, StatementStatus, ValidTime};
pub use time::{RecordedTime, ValidPeriod};

/// Re-export of common chrono types used across the domain.
pub use chrono::{DateTime, Utc};
