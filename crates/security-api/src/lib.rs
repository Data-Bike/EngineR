//! Security API: contract + `LocalSecurityClient` + internal Rocket
//! controllers (docs/08 §1, docs/09, ADR-008).
//!
//! The contract is identical for in-process local transport and remote mTLS
//! HTTP. In the hot path the local client calls the PDP crate directly (no
//! self-HTTP); the internal Rocket controllers expose the same contract to
//! other services over mTLS.

#![forbid(unsafe_code)]

pub mod client;
pub mod contract;
pub mod routes;

pub use client::{LocalSecurityClient, SecurityClient};
pub use contract::{
    ActionDecisionRequest, ActionDecisionResponse, DecisionEffectDto, QueryDecisionRequest,
    QueryDecisionResponse, SecurityContextDto,
};
pub use routes::local_client;
