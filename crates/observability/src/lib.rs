//! Observability: tracing, metrics and audit helpers (docs/21).
#![forbid(unsafe_code)]

pub mod audit;
pub mod tracing_init;

pub use audit::{AuditAction, AuditEvent, AuditOutcome};
pub use tracing_init::init;
