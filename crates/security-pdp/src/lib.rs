//! Policy Decision Point (docs/08 §4, docs/09).
//!
//! Evaluation:
//! 1. verify tenant/session/auth strength;
//! 2. collect memberships on policy snapshot;
//! 3. check capability;
//! 4. collect applicable data rules;
//! 5. drop rules with false condition / expired;
//! 6. apply deny-overrides;
//! 7. form allowed set + obligations;
//! 8. constrain the request/command;
//! 9. record decision audit.
//!
//! Default deny (ADR-011). Allow-by-absence is forbidden.

#![forbid(unsafe_code)]

pub mod engine;

pub use engine::{AuthorizationRequest, AuthorizationResource, PdpEngine, PdpError, PdpSnapshot};
