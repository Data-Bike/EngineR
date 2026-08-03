//! PDP evaluation engine (docs/08 §4, docs/09 §2-5).
//!
//! The engine is pure: it takes a snapshot of policies + memberships and an
//! authorization request, and returns a decision. Persistence/caching is the
//! caller's responsibility. This makes the engine trivially testable and
//! deterministic — the same inputs always produce the same decision.

use enginer_domain::{DecisionId, PolicyId, PrincipalId, TenantId};
use enginer_security_domain::{
    decision::{
        DataObligations, DecisionEffect, Obligations, QueryObligations, SecurityDecision,
    },
    policy::{PolicyRule, PolicyRuleEffect},
    principal::{GroupClass, PrincipalGroupMembership},
    CapabilityAction,
};
use chrono::{DateTime, Duration, Utc};
use thiserror::Error;

/// PDP error (fail closed).
#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum PdpError {
    #[error("context_invalid: {message}")]
    ContextInvalid { message: String },
    #[error("policy_unavailable")]
    PolicyUnavailable,
}

/// The resource being authorized.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuthorizationResource {
    /// Capability action (system right).
    pub action: CapabilityAction,
    /// Optional resource id (e.g. report definition).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_id: Option<uuid::Uuid>,
    /// Root type ids for query decisions.
    #[serde(default)]
    pub root_type_ids: Vec<uuid::Uuid>,
    /// Selected predicate ids for query decisions.
    #[serde(default)]
    pub selected_predicate_ids: Vec<uuid::Uuid>,
}

/// An authorization request.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AuthorizationRequest {
    pub tenant_id: TenantId,
    pub principal_id: PrincipalId,
    pub auth_strength: String,
    pub purpose: String,
    pub time: DateTime<Utc>,
    pub resource: AuthorizationResource,
}

/// A frozen snapshot of the policies and memberships the engine evaluates
/// against. Built by the caller (e.g. from a policy cache) and immutable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdpSnapshot {
    pub policy_revision: i64,
    pub memberships: Vec<PrincipalGroupMembership>,
    pub rules: Vec<PolicyRule>,
}

impl PdpSnapshot {
    /// Empty snapshot (default-deny for everyone).
    pub fn empty(policy_revision: i64) -> Self {
        Self {
            policy_revision,
            memberships: Vec::new(),
            rules: Vec::new(),
        }
    }

    /// Groups the principal is a member of at `time`, filtered by class.
    pub fn active_groups(
        &self,
        principal: PrincipalId,
        class: GroupClass,
        time: DateTime<Utc>,
    ) -> Vec<uuid::Uuid> {
        self.memberships
            .iter()
            .filter(|m| m.principal_id == principal && m.group_class == class)
            .filter(|m| m.valid_from <= time && m.valid_to.map_or(true, |to| to >= time))
            .map(|m| m.group_id.raw())
            .collect()
    }
}

/// The pure PDP engine.
#[derive(Debug, Clone, Default)]
pub struct PdpEngine;

impl PdpEngine {
    /// Evaluate an action authorization request (docs/09 §4).
    pub fn evaluate_action(
        &self,
        snap: &PdpSnapshot,
        req: &AuthorizationRequest,
    ) -> Result<SecurityDecision, PdpError> {
        if req.tenant_id.raw().is_nil() {
            return Err(PdpError::ContextInvalid {
                message: "tenant_id is nil".to_string(),
            });
        }

        let cap_groups = snap.active_groups(req.principal_id, GroupClass::Capability, req.time);
        let action_str = req.resource.action.as_str();

        // Collect applicable capability rules.
        let applicable: Vec<&PolicyRule> = snap
            .rules
            .iter()
            .filter(|r| {
                r.tenant_id == req.tenant_id
                    && r.action == action_str
                    && cap_groups.contains(&r.group_id.raw())
                    && r.valid_from <= req.time
                    && r.valid_to.map_or(true, |to| to >= req.time)
            })
            .collect();

        // Deny-overrides (ADR-011).
        let has_deny = applicable.iter().any(|r| r.effect == PolicyRuleEffect::Deny);
        let has_allow = applicable.iter().any(|r| r.effect == PolicyRuleEffect::Allow);

        let effect = if has_deny || !has_allow {
            // default deny when no explicit allow, or any deny
            DecisionEffect::Deny
        } else {
            DecisionEffect::AllowWithObligations
        };

        let obligations = if matches!(effect, DecisionEffect::AllowWithObligations) {
            Obligations {
                data: Some(DataObligations {
                    max_rows: None,
                    watermark: None,
                    require_step_up: false,
                    audit_level: "full".to_string(),
                }),
                query: Some(empty_query_obligations()),
                mutation: None,
            }
        } else {
            Obligations {
                data: None,
                query: None,
                mutation: None,
            }
        };

        Ok(SecurityDecision {
            decision_id: DecisionId::new(),
            effect,
            policy_revision: snap.policy_revision,
            obligations,
            expires_at: req.time + Duration::seconds(60),
            policy_ids: applicable.iter().map(|r| r.policy_id).collect(),
        })
    }
}

fn empty_query_obligations() -> QueryObligations {
    QueryObligations {
        allowed_root_type_ids: Vec::new(),
        denied_type_ids: Vec::new(),
        allowed_predicate_ids: Vec::new(),
        denied_predicate_ids: Vec::new(),
        allowed_relations: Vec::new(),
        mandatory_filters: Vec::new(),
        masks: Vec::new(),
        max_page_size: 50,
        max_graph_depth: 2,
        max_graph_nodes: 300,
        aggregates: "allow".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use enginer_domain::{GroupId, PolicyId};
    use enginer_security_domain::policy::ResourceSelector;

    fn rule(
        tenant: TenantId,
        group: GroupId,
        effect: PolicyRuleEffect,
        action: &str,
        revision: i64,
    ) -> PolicyRule {
        PolicyRule {
            policy_id: PolicyId::new(),
            tenant_id: tenant,
            effect,
            group_id: group,
            action: action.to_string(),
            resource: ResourceSelector {
                type_id: None,
                entity_id: None,
                predicate_ids: Vec::new(),
                classification: None,
                business_unit_scope: None,
            },
            row_condition: None,
            field_mask: None,
            priority: 0,
            valid_from: Utc::now() - Duration::days(1),
            valid_to: None,
            policy_revision: revision,
            reason: None,
        }
    }

    fn membership(principal: PrincipalId, group: GroupId, class: GroupClass) -> PrincipalGroupMembership {
        PrincipalGroupMembership {
            principal_id: principal,
            group_id: group,
            group_class: class,
            valid_from: Utc::now() - Duration::days(1),
            valid_to: None,
            groups_revision: 1,
        }
    }

    #[test]
    fn default_deny_when_no_rules() {
        let snap = PdpSnapshot::empty(1);
        let req = AuthorizationRequest {
            tenant_id: TenantId::new(),
            principal_id: PrincipalId::new(),
            auth_strength: "single".to_string(),
            purpose: "test".to_string(),
            time: Utc::now(),
            resource: AuthorizationResource {
                action: CapabilityAction::EntityCreate,
                resource_id: None,
                root_type_ids: Vec::new(),
                selected_predicate_ids: Vec::new(),
            },
        };
        let dec = PdpEngine.evaluate_action(&snap, &req).unwrap();
        assert_eq!(dec.effect, DecisionEffect::Deny);
    }

    #[test]
    fn allow_when_explicit_allow_and_no_deny() {
        let tenant = TenantId::new();
        let principal = PrincipalId::new();
        let group = GroupId::new();
        let snap = PdpSnapshot {
            policy_revision: 1,
            memberships: vec![membership(principal, group, GroupClass::Capability)],
            rules: vec![rule(
                tenant,
                group,
                PolicyRuleEffect::Allow,
                CapabilityAction::EntityCreate.as_str(),
                1,
            )],
        };
        let req = AuthorizationRequest {
            tenant_id: tenant,
            principal_id: principal,
            auth_strength: "single".to_string(),
            purpose: "test".to_string(),
            time: Utc::now(),
            resource: AuthorizationResource {
                action: CapabilityAction::EntityCreate,
                resource_id: None,
                root_type_ids: Vec::new(),
                selected_predicate_ids: Vec::new(),
            },
        };
        let dec = PdpEngine.evaluate_action(&snap, &req).unwrap();
        assert!(matches!(dec.effect, DecisionEffect::AllowWithObligations));
    }

    #[test]
    fn deny_overrides_allow() {
        let tenant = TenantId::new();
        let principal = PrincipalId::new();
        let group = GroupId::new();
        let snap = PdpSnapshot {
            policy_revision: 1,
            memberships: vec![membership(principal, group, GroupClass::Capability)],
            rules: vec![
                rule(
                    tenant,
                    group,
                    PolicyRuleEffect::Allow,
                    CapabilityAction::EntityCreate.as_str(),
                    1,
                ),
                rule(
                    tenant,
                    group,
                    PolicyRuleEffect::Deny,
                    CapabilityAction::EntityCreate.as_str(),
                    1,
                ),
            ],
        };
        let req = AuthorizationRequest {
            tenant_id: tenant,
            principal_id: principal,
            auth_strength: "single".to_string(),
            purpose: "test".to_string(),
            time: Utc::now(),
            resource: AuthorizationResource {
                action: CapabilityAction::EntityCreate,
                resource_id: None,
                root_type_ids: Vec::new(),
                selected_predicate_ids: Vec::new(),
            },
        };
        let dec = PdpEngine.evaluate_action(&snap, &req).unwrap();
        assert_eq!(dec.effect, DecisionEffect::Deny);
    }

    #[test]
    fn data_group_does_not_grant_capability() {
        let tenant = TenantId::new();
        let principal = PrincipalId::new();
        let cap_group = GroupId::new();
        let data_group = GroupId::new();
        // An allow rule attached to a DATA group must not grant a capability.
        let snap = PdpSnapshot {
            policy_revision: 1,
            memberships: vec![membership(principal, data_group, GroupClass::Data)],
            rules: vec![rule(
                tenant,
                data_group,
                PolicyRuleEffect::Allow,
                CapabilityAction::EntityCreate.as_str(),
                1,
            )],
        };
        let req = AuthorizationRequest {
            tenant_id: tenant,
            principal_id: principal,
            auth_strength: "single".to_string(),
            purpose: "test".to_string(),
            time: Utc::now(),
            resource: AuthorizationResource {
                action: CapabilityAction::EntityCreate,
                resource_id: None,
                root_type_ids: Vec::new(),
                selected_predicate_ids: Vec::new(),
            },
        };
        let dec = PdpEngine.evaluate_action(&snap, &req).unwrap();
        // principal is not in any capability group, so no applicable allow rule
        assert_eq!(dec.effect, DecisionEffect::Deny);
        let _ = cap_group;
    }
}
