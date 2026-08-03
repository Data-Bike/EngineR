//! Schema registry: draft → validate → publish (docs/12).
//!
//! Published revisions are immutable (ADR-004, docs/12 §1). Validation checks
//! aliases/IDs, domain/range/cardinality, inheritance cycles, inverse/symmetric
//! coherence, identity normalizers, Unicode/confusables, etc. (docs/12 §2).
//!
//! Schema bundles (docs/03 §10) are signed JSON packages loaded via Admin API.
//! The [`bundle`] module provides a loader that converts bundle files into a
//! `SchemaDraft` ready for validation and publish.

#![forbid(unsafe_code)]

pub mod bundle;
pub mod draft;
pub mod validate;

pub use bundle::{BundleLoadError, LoadedBundle};
pub use draft::{SchemaDraft, SchemaDraftError};
pub use validate::{Severity, ValidationFinding, ValidationReport};
