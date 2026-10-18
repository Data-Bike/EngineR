//! PostgreSQL persistence adapter (docs/10).
//!
//! All access goes through sqlx. RLS context (`set_config`) is set
//! per-transaction and reset before return (docs/10 §17). Dynamic business
//! schema never creates arbitrary DDL in the request path (ADR-006).

#![forbid(unsafe_code)]

pub mod entity_repo;
pub mod rls;
pub mod schema_repo;
pub mod security_repo;
pub mod statement_repo;

pub use entity_repo::{PgEntityRepository, PgIdentityKeyRepository, PgIdentityPolicyRepository};
pub use rls::{RlsContext, RlsContextError};
pub use schema_repo::PgSchemaRepository;
pub use security_repo::PgSecuritySnapshotProvider;
pub use statement_repo::PgStatementRepository;
