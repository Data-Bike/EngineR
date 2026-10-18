//! Security: PDP/PEP, ABAC, capabilities, audit, classification (docs/08).
//!
//! Security is implemented in separate crates (security-domain, security-pdp,
//! security-api). The main domain does not decide access. Every controller/
//! use case/query/export/worker passes through a Policy Enforcement Point
//! (PEP) (docs/08 §1).
//!
//! Two classes of rights:
//! - System capabilities: access to functions (docs/08 §2.1)
//! - Data permissions: access to data (docs/08 §2.2)
//!
//! Default deny. Explicit deny overrides allow. Break-glass is a separate
//! step-up flow, not a normal priority (docs/08 §3).

#![forbid(unsafe_code)]

pub mod abac;
pub mod audit;
pub mod capability;
pub mod classification;
pub mod error;
pub mod pdp;

pub use abac::{AttributeValue, DataPermission, DataRule, Effect, PolicyRule, ResourceSelector};
pub use audit::{AuditEntry, AuditKind, AuditQuery, AuditRetention, TamperChain};
pub use capability::{Capability, CapabilityGroup, CapabilityGroupKind, CapabilityKind};
pub use classification::{ClassificationLevel, FieldPolicy, FieldPolicyKind, MaskStrategy};
pub use error::{SecurityError, SecurityResult};
pub use pdp::{
    Decision, DecisionContext, DecisionKind, LocalPdpClient, PdpClient, PolicySnapshot, Principal,
    PrincipalKind, SessionInfo,
};
