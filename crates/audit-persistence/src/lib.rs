//! Audit persistence: repository trait + PostgreSQL impl + tamper-evident
//! chain (docs/08 §11).
//!
//! Audit rows are insert-only from the application role; no UPDATE/DELETE
//! is granted to enginer_app. A separate retention role handles archival
//! (docs/08 §11, §15).
//!
//! Tamper-evident chain: each entry's hash includes the previous entry's
//! hash, forming a chain. Checkpoints periodically persist the latest hash
//! for verification (docs/08 §11).

#![forbid(unsafe_code)]

pub mod chain;
pub mod error;
pub mod repository;

pub use chain::{ChainHasher, ChainState};
pub use error::{AuditPersistenceError, AuditPersistenceResult};
pub use repository::{
    AuditQuery as AuditRepoQuery, AuditRecord, AuditRepository, ChainCheckpoint,
    ChainVerificationResult, PgAuditRepository, RetentionPolicyRecord,
};
