//! Application layer: use cases, ports and request context (docs/02 §3, docs/05 §5).
#![forbid(unsafe_code)]

pub mod context;
pub mod error;
pub mod ports;
pub mod use_cases;

#[cfg(test)]
mod test_mocks;

pub use context::{AuthStrength, RequestContext};
pub use error::ApplicationError;
pub use ports::{
    AuditSink, Clock, EntityRepository, IdGenerator, IdentityKeyRepository,
    IdentityPolicyRepository, Outbox, SchemaRepository, StatementRepository,
};
