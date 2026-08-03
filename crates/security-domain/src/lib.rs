//! Security domain: principals, capabilities, data scopes, decisions
//! (docs/08, docs/09).
#![forbid(unsafe_code)]

pub mod capabilities;
pub mod decision;
pub mod policy;
pub mod principal;

pub use capabilities::{Capability, CapabilityAction};
pub use decision::{
    DataObligations, DecisionEffect, FieldMask, FieldPolicy, MandatoryFilter, MutationObligations,
    Obligations, QueryObligations, SecurityDecision,
};
pub use policy::{PolicyRule, PolicyRuleEffect, ResourceSelector};
pub use principal::{GroupClass, Principal, PrincipalGroupMembership};
