//! PDP: Policy Decision Point and PEP enforcement (docs/08 §1, §4).
//!
//! Evaluation (docs/08 §4):
//! 1. Check tenant, session, auth strength.
//! 2. Gather memberships on policy snapshot.
//! 3. Check capability.
//! 4. Gather applicable data rules.
//! 5. Discard rules with false condition/expired.
//! 6. Apply deny-overrides.
//! 7. Form allowed set and obligations.
//! 8. Constrain query/command.
//! 9. Record decision audit with policy IDs/revision.
//!
//! Default deny. Fail closed (docs/08 §4, §15).

use crate::abac::{DataPermission, DataRule, Effect};
use crate::capability::{Capability, CapabilityGroup};
use crate::error::{SecurityError, SecurityResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Principal kind (docs/08 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    /// User principal.
    User,
    /// Service account principal.
    ServiceAccount,
    /// System principal (internal).
    System,
}

/// Principal (docs/08 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    /// Principal ID.
    pub principal_id: String,
    /// Principal kind.
    pub kind: PrincipalKind,
    /// Tenant ID.
    pub tenant_id: String,
    /// Group IDs the principal is a member of.
    pub group_ids: Vec<String>,
    /// Authentication strength: "password", "mfa", "step_up".
    pub auth_strength: String,
    /// Purpose of use (docs/08 §2.2).
    pub purpose: Option<String>,
}

/// Session info (docs/08 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID.
    pub session_id: String,
    /// Device ID (if known).
    pub device_id: Option<String>,
    /// Client IP address.
    pub ip_address: String,
    /// User agent.
    pub user_agent: Option<String>,
    /// Whether the session has MFA.
    pub mfa_verified: bool,
    /// Whether the session is step-up.
    pub step_up: bool,
    /// Session risk score (0.0 - 1.0, stored as string for precision).
    pub risk_score: String,
}

/// Policy snapshot (docs/08 §10).
///
/// Pinned policy revision for consistent evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicySnapshot {
    /// Policy revision ID.
    pub revision_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Groups revision ID (for membership).
    pub groups_revision_id: String,
    /// Capabilities (name → capability).
    pub capabilities: HashMap<String, Capability>,
    /// Capability groups (group_id → group).
    pub capability_groups: HashMap<String, CapabilityGroup>,
    /// Data rules.
    pub data_rules: Vec<DataRule>,
    /// Published at.
    pub published_at: chrono::DateTime<chrono::Utc>,
}

impl PolicySnapshot {
    /// Create a new empty policy snapshot.
    pub fn new(revision_id: &str, tenant_id: &str, groups_revision_id: &str) -> Self {
        Self {
            revision_id: revision_id.to_string(),
            tenant_id: tenant_id.to_string(),
            groups_revision_id: groups_revision_id.to_string(),
            capabilities: HashMap::new(),
            capability_groups: HashMap::new(),
            data_rules: Vec::new(),
            published_at: chrono::Utc::now(),
        }
    }

    /// Add a capability.
    pub fn add_capability(&mut self, cap: Capability) {
        self.capabilities.insert(cap.name.clone(), cap);
    }

    /// Add a capability group.
    pub fn add_group(&mut self, group: CapabilityGroup) {
        self.capability_groups.insert(group.group_id.clone(), group);
    }

    /// Add a data rule.
    pub fn add_data_rule(&mut self, rule: DataRule) {
        self.data_rules.push(rule);
    }

    /// Get all capabilities for a principal (from all groups).
    pub fn principal_capabilities(&self, principal: &Principal) -> Vec<&Capability> {
        let mut caps: Vec<&Capability> = Vec::new();
        for group_id in &principal.group_ids {
            if let Some(group) = self.capability_groups.get(group_id) {
                for cap_name in &group.capabilities {
                    if let Some(cap) = self.capabilities.get(cap_name) {
                        if !caps.iter().any(|c| c.name == cap.name) {
                            caps.push(cap);
                        }
                    }
                }
            }
        }
        caps
    }

    /// Check if a principal has a capability.
    pub fn has_capability(&self, principal: &Principal, capability: &str) -> bool {
        self.principal_capabilities(principal)
            .iter()
            .any(|c| c.name == capability)
    }

    /// Get applicable data rules for a principal and action.
    pub fn applicable_data_rules(
        &self,
        principal: &Principal,
        action: &str,
        at: chrono::DateTime<chrono::Utc>,
    ) -> Vec<&DataRule> {
        let mut rules: Vec<&DataRule> = Vec::new();
        // Direct subject rules
        for rule in &self.data_rules {
            if rule.matches(&principal.principal_id, action) && rule.is_valid_at(at) {
                rules.push(rule);
            }
        }
        // Group-based rules
        for group_id in &principal.group_ids {
            for rule in &self.data_rules {
                if rule.matches(group_id, action) && rule.is_valid_at(at) {
                    if !rules.iter().any(|r| r.rule_id == rule.rule_id) {
                        rules.push(rule);
                    }
                }
            }
        }
        // Sort by priority (highest first)
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }
}

/// Decision kind (docs/08 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    /// Access allowed.
    Allow,
    /// Access denied (default deny, docs/08 §4).
    Deny,
    /// Access denied, break-glass required (docs/08 §13).
    BreakGlassRequired,
    /// Step-up authentication required (docs/08 §9).
    StepUpRequired,
}

/// Decision context (docs/08 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Action being authorized.
    pub action: String,
    /// Resource type (if applicable).
    pub resource_type: Option<String>,
    /// Resource ID (if applicable).
    pub resource_id: Option<String>,
    /// Requested fields (if applicable).
    pub requested_fields: Vec<String>,
    /// Requested relations (if applicable).
    pub requested_relations: Vec<String>,
    /// Whether this is a high-risk mutation (bypasses cache, docs/08 §10).
    pub high_risk: bool,
}

/// A policy decision (docs/08 §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// Decision ID (UUID).
    pub decision_id: String,
    /// Decision kind.
    pub kind: DecisionKind,
    /// Principal.
    pub principal_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Action.
    pub action: String,
    /// Policy revision used.
    pub policy_revision_id: String,
    /// Rule IDs that contributed to the decision.
    pub rule_ids: Vec<String>,
    /// Resolved data permissions (if allowed).
    pub data_permission: Option<DataPermission>,
    /// Obligations (e.g. "log", "watermark").
    pub obligations: Vec<String>,
    /// Reason for the decision.
    pub reason: String,
    /// Whether the decision was cached (docs/08 §10).
    pub cached: bool,
    /// When the decision was made.
    pub decided_at: chrono::DateTime<chrono::Utc>,
}

impl Decision {
    /// Create an allow decision.
    pub fn allow(
        principal_id: &str,
        tenant_id: &str,
        action: &str,
        policy_revision_id: &str,
        rule_ids: Vec<String>,
        data_permission: DataPermission,
        obligations: Vec<String>,
    ) -> Self {
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            kind: DecisionKind::Allow,
            principal_id: principal_id.to_string(),
            tenant_id: tenant_id.to_string(),
            action: action.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            rule_ids,
            data_permission: Some(data_permission),
            obligations,
            reason: "allowed".to_string(),
            cached: false,
            decided_at: chrono::Utc::now(),
        }
    }

    /// Create a deny decision.
    pub fn deny(
        principal_id: &str,
        tenant_id: &str,
        action: &str,
        policy_revision_id: &str,
        rule_ids: Vec<String>,
        reason: &str,
    ) -> Self {
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            kind: DecisionKind::Deny,
            principal_id: principal_id.to_string(),
            tenant_id: tenant_id.to_string(),
            action: action.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            rule_ids,
            data_permission: None,
            obligations: Vec::new(),
            reason: reason.to_string(),
            cached: false,
            decided_at: chrono::Utc::now(),
        }
    }

    /// Create a break-glass required decision.
    pub fn break_glass_required(
        principal_id: &str,
        tenant_id: &str,
        action: &str,
        policy_revision_id: &str,
        reason: &str,
    ) -> Self {
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            kind: DecisionKind::BreakGlassRequired,
            principal_id: principal_id.to_string(),
            tenant_id: tenant_id.to_string(),
            action: action.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            rule_ids: Vec::new(),
            data_permission: None,
            obligations: Vec::new(),
            reason: reason.to_string(),
            cached: false,
            decided_at: chrono::Utc::now(),
        }
    }

    /// Create a step-up required decision.
    pub fn step_up_required(
        principal_id: &str,
        tenant_id: &str,
        action: &str,
        policy_revision_id: &str,
        reason: &str,
    ) -> Self {
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            kind: DecisionKind::StepUpRequired,
            principal_id: principal_id.to_string(),
            tenant_id: tenant_id.to_string(),
            action: action.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            rule_ids: Vec::new(),
            data_permission: None,
            obligations: Vec::new(),
            reason: reason.to_string(),
            cached: false,
            decided_at: chrono::Utc::now(),
        }
    }

    /// Check if the decision is allowed.
    pub fn is_allowed(&self) -> bool {
        self.kind == DecisionKind::Allow
    }

    /// Check if the decision is denied.
    pub fn is_denied(&self) -> bool {
        self.kind == DecisionKind::Deny
    }
}

/// PDP client trait (docs/08 §1).
///
/// Hot path uses LocalSecurityClient by the same JSON contract without
/// loopback HTTP. RemoteSecurityClient is used when PDP is separated or
/// for other systems.
pub trait PdpClient: Send + Sync {
    /// Evaluate a policy decision.
    fn evaluate(
        &self,
        principal: &Principal,
        session: &SessionInfo,
        context: &DecisionContext,
    ) -> SecurityResult<Decision>;

    /// Get the current policy snapshot for a tenant.
    fn policy_snapshot(&self, tenant_id: &str) -> SecurityResult<PolicySnapshot>;
}

/// Local PDP client (docs/08 §1).
///
/// Evaluates decisions in-process using a policy snapshot.
/// Used in the hot path to avoid loopback HTTP.
pub struct LocalPdpClient {
    snapshot: PolicySnapshot,
}

impl LocalPdpClient {
    /// Create a new local PDP client with a policy snapshot.
    pub fn new(snapshot: PolicySnapshot) -> Self {
        Self { snapshot }
    }

    /// Evaluate a decision using the policy snapshot.
    pub fn evaluate_internal(
        &self,
        principal: &Principal,
        session: &SessionInfo,
        context: &DecisionContext,
    ) -> SecurityResult<Decision> {
        // Step 1: Check tenant (docs/08 §4)
        if principal.tenant_id != self.snapshot.tenant_id {
            return Ok(Decision::deny(
                &principal.principal_id,
                &principal.tenant_id,
                &context.action,
                &self.snapshot.revision_id,
                Vec::new(),
                "tenant mismatch",
            ));
        }

        // Step 3: Check capability (docs/08 §4)
        let cap_name = &context.action;
        if !self.snapshot.has_capability(principal, cap_name) {
            return Ok(Decision::deny(
                &principal.principal_id,
                &principal.tenant_id,
                &context.action,
                &self.snapshot.revision_id,
                Vec::new(),
                &format!("capability_missing: {}", cap_name),
            ));
        }

        // Check if step-up is required (docs/08 §9)
        if let Some(cap) = self.snapshot.capabilities.get(cap_name) {
            if cap.requires_step_up && !session.step_up {
                return Ok(Decision::step_up_required(
                    &principal.principal_id,
                    &principal.tenant_id,
                    &context.action,
                    &self.snapshot.revision_id,
                    "step-up MFA required for this action",
                ));
            }
        }

        // Step 4-6: Gather data rules, discard invalid, apply deny-overrides
        let now = chrono::Utc::now();
        let rules = self.snapshot.applicable_data_rules(principal, &context.action, now);

        // Check for deny rules (deny-overrides, docs/08 §3)
        let deny_rules: Vec<&DataRule> = rules.iter().filter(|r| r.is_deny()).copied().collect();
        if !deny_rules.is_empty() {
            let rule_ids: Vec<String> = deny_rules.iter().map(|r| r.rule_id.clone()).collect();
            return Ok(Decision::deny(
                &principal.principal_id,
                &principal.tenant_id,
                &context.action,
                &self.snapshot.revision_id,
                rule_ids,
                "denied by explicit deny rule",
            ));
        }

        // Step 7: Form allowed set and obligations
        let allow_rules: Vec<&DataRule> = rules.iter().filter(|r| !r.is_deny()).copied().collect();
        let rule_ids: Vec<String> = allow_rules.iter().map(|r| r.rule_id.clone()).collect();
        let obligations: Vec<String> = allow_rules
            .iter()
            .flat_map(|r| r.obligations.clone())
            .collect();

        // Build data permission from allow rules
        let mut data_perm = DataPermission::default();
        for rule in &allow_rules {
            if let Some(entity_type) = &rule.resource.entity_type {
                if !data_perm.allowed_types.contains(entity_type) {
                    data_perm.allowed_types.push(entity_type.clone());
                }
            }
            for field in &rule.resource.fields {
                if let Some(entity_type) = &rule.resource.entity_type {
                    if let Some(entry) =
                        data_perm.allowed_fields.iter_mut().find(|(t, _)| t == entity_type)
                    {
                        if !entry.1.contains(field) {
                            entry.1.push(field.clone());
                        }
                    } else {
                        data_perm
                            .allowed_fields
                            .push((entity_type.clone(), vec![field.clone()]));
                    }
                }
            }
            for rel in &rule.resource.relations {
                if !data_perm.allowed_relations.contains(rel) {
                    data_perm.allowed_relations.push(rel.clone());
                }
            }
            for class in &rule.resource.classifications {
                if !data_perm.allowed_classifications.contains(class) {
                    data_perm.allowed_classifications.push(class.clone());
                }
            }
            if let Some(row_cond) = &rule.resource.row_condition {
                data_perm.row_condition = Some(row_cond.clone());
            }
        }

        // Step 9: Return decision
        Ok(Decision::allow(
            &principal.principal_id,
            &principal.tenant_id,
            &context.action,
            &self.snapshot.revision_id,
            rule_ids,
            data_perm,
            obligations,
        ))
    }
}

impl PdpClient for LocalPdpClient {
    fn evaluate(
        &self,
        principal: &Principal,
        session: &SessionInfo,
        context: &DecisionContext,
    ) -> SecurityResult<Decision> {
        self.evaluate_internal(principal, session, context)
    }

    fn policy_snapshot(&self, tenant_id: &str) -> SecurityResult<PolicySnapshot> {
        if tenant_id != self.snapshot.tenant_id {
            return Err(SecurityError::NotFound {
                message: "policy snapshot not found for tenant".to_string(),
            });
        }
        Ok(self.snapshot.clone())
    }
}
