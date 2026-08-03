//! Security crate tests (docs/08).

#![cfg(test)]

use enginer_security::{
    AuditEntry, AuditKind, AuditQuery, AuditRetention, Capability, CapabilityGroup,
    CapabilityGroupKind, CapabilityKind, ClassificationLevel, DataPermission, DataRule,
    Decision, DecisionContext, DecisionKind, Effect, FieldPolicy, FieldPolicyKind,
    LocalPdpClient, MaskStrategy, PdpClient, PolicySnapshot, Principal, PrincipalKind,
    ResourceSelector, SessionInfo, TamperChain,
};
use std::collections::HashMap;

// --- Capability tests ---

#[test]
fn capability_group_has_capability() {
    let group = CapabilityGroup {
        group_id: "g1".to_string(),
        name: "Editors".to_string(),
        kind: CapabilityGroupKind::System,
        capabilities: vec!["entity.create".to_string(), "entity.edit".to_string()],
        description_key: "group.editors".to_string(),
        owner: "team-001".to_string(),
    };
    assert!(group.has_capability("entity.create"));
    assert!(!group.has_capability("entity.retire"));
}

#[test]
fn capability_group_add_remove() {
    let mut group = CapabilityGroup {
        group_id: "g1".to_string(),
        name: "Editors".to_string(),
        kind: CapabilityGroupKind::System,
        capabilities: vec!["entity.create".to_string()],
        description_key: "group.editors".to_string(),
        owner: "team-001".to_string(),
    };
    group.add_capability("entity.edit");
    assert_eq!(group.capabilities.len(), 2);
    // Idempotent
    group.add_capability("entity.edit");
    assert_eq!(group.capabilities.len(), 2);
    group.remove_capability("entity.edit");
    assert_eq!(group.capabilities.len(), 1);
    assert!(!group.has_capability("entity.edit"));
}

#[test]
fn standard_system_capabilities_count() {
    let caps = enginer_security::capability::standard_system_capabilities();
    assert!(caps.len() >= 20);
    assert!(caps.iter().all(|c| c.kind == CapabilityKind::System));
}

#[test]
fn standard_capabilities_include_key_ones() {
    let caps = enginer_security::capability::standard_system_capabilities();
    let names: Vec<&str> = caps.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"entity.create"));
    assert!(names.contains(&"entity.merge"));
    assert!(names.contains(&"report.export"));
    assert!(names.contains(&"schema.publish"));
    assert!(names.contains(&"security.manage"));
    assert!(names.contains(&"audit.read"));
}

#[test]
fn high_risk_capabilities_bypass_cache() {
    let caps = enginer_security::capability::standard_system_capabilities();
    // Per docs/08 §10, high_risk capabilities bypass policy cache
    let high_risk_count = caps.iter().filter(|c| c.high_risk).count();
    assert!(high_risk_count >= 5, "expected at least 5 high_risk capabilities");
}

#[test]
fn step_up_capabilities_include_key_ones() {
    let caps = enginer_security::capability::standard_system_capabilities();
    // Per docs/08 §9, step-up is required for publish, export, security, break-glass
    let step_up_names: Vec<&str> = caps
        .iter()
        .filter(|c| c.requires_step_up)
        .map(|c| c.name.as_str())
        .collect();
    assert!(step_up_names.contains(&"schema.publish"));
    assert!(step_up_names.contains(&"report.export"));
    assert!(step_up_names.contains(&"security.manage"));
    assert!(step_up_names.contains(&"security.simulate"));
}

// --- Classification tests ---

#[test]
fn classification_level_rank() {
    assert_eq!(ClassificationLevel::Public.rank(), 0);
    assert_eq!(ClassificationLevel::Internal.rank(), 1);
    assert_eq!(ClassificationLevel::Restricted.rank(), 2);
    assert_eq!(ClassificationLevel::Confidential.rank(), 3);
    assert_eq!(ClassificationLevel::Secret.rank(), 4);
}

#[test]
fn classification_level_is_allowed_with() {
    assert!(ClassificationLevel::Public.is_allowed_with(ClassificationLevel::Restricted));
    assert!(ClassificationLevel::Restricted.is_allowed_with(ClassificationLevel::Restricted));
    assert!(!ClassificationLevel::Confidential.is_allowed_with(ClassificationLevel::Restricted));
}

#[test]
fn classification_level_from_str() {
    assert_eq!(ClassificationLevel::from_str("public"), Some(ClassificationLevel::Public));
    assert_eq!(ClassificationLevel::from_str("RESTRICTED"), Some(ClassificationLevel::Restricted));
    assert_eq!(ClassificationLevel::from_str("unknown"), None);
}

#[test]
fn classification_level_as_str() {
    assert_eq!(ClassificationLevel::Public.as_str(), "public");
    assert_eq!(ClassificationLevel::Secret.as_str(), "secret");
}

#[test]
fn field_policy_allow() {
    let policy = FieldPolicy::allow("person.inn");
    assert_eq!(policy.kind, FieldPolicyKind::Allow);
    assert_eq!(policy.apply("1234567890"), "1234567890");
}

#[test]
fn field_policy_omit() {
    let policy = FieldPolicy::omit("person.inn");
    assert_eq!(policy.kind, FieldPolicyKind::Omit);
    assert_eq!(policy.apply("1234567890"), "");
}

#[test]
fn field_policy_null_mask() {
    let policy = FieldPolicy::null_mask("person.inn", true);
    assert_eq!(policy.kind, FieldPolicyKind::NullMask);
    assert_eq!(policy.apply("1234567890"), "null");
    assert!(policy.mark_in_meta);
}

#[test]
fn field_policy_partial_mask() {
    let policy = FieldPolicy::partial_mask("person.inn", 2, 2, "*", true);
    assert_eq!(policy.kind, FieldPolicyKind::PartialMask);
    assert_eq!(policy.apply("1234567890"), "12******90");
}

#[test]
fn mask_strategy_partial_mask_short_value() {
    let mask = MaskStrategy::partial_mask(2, 2, "*");
    // Value shorter than prefix+suffix should be fully masked
    assert_eq!(mask.apply("abc"), "***");
}

#[test]
fn mask_strategy_tokenize() {
    let mask = MaskStrategy::tokenize("key-001");
    let tokenized = mask.apply("1234567890");
    assert!(tokenized.starts_with("tok_"));
}

// --- ABAC / DataPermission tests ---

#[test]
fn data_permission_is_type_allowed() {
    let mut perm = DataPermission::default();
    perm.allowed_types = vec!["legal_entity".to_string(), "holding".to_string()];
    perm.denied_types = vec!["person".to_string()];
    assert!(perm.is_type_allowed("legal_entity"));
    assert!(perm.is_type_allowed("holding"));
    assert!(!perm.is_type_allowed("person"));
    assert!(!perm.is_type_allowed("unknown_type"));
}

#[test]
fn data_permission_is_type_allowed_empty_allow_list() {
    let mut perm = DataPermission::default();
    perm.denied_types = vec!["person".to_string()];
    // Empty allow list = all allowed (except denied)
    assert!(perm.is_type_allowed("legal_entity"));
    assert!(!perm.is_type_allowed("person"));
}

#[test]
fn data_permission_is_field_allowed() {
    let mut perm = DataPermission::default();
    perm.allowed_fields = vec![("legal_entity".to_string(), vec!["full_name".to_string(), "inn".to_string()])];
    perm.denied_fields = vec![("legal_entity".to_string(), vec!["ogrn".to_string()])];
    assert!(perm.is_field_allowed("legal_entity", "full_name"));
    assert!(perm.is_field_allowed("legal_entity", "inn"));
    assert!(!perm.is_field_allowed("legal_entity", "ogrn"));
}

#[test]
fn data_permission_is_field_allowed_empty_fields() {
    let mut perm = DataPermission::default();
    perm.allowed_fields = vec![("legal_entity".to_string(), vec![])];
    // Empty field list = all fields allowed for that type
    assert!(perm.is_field_allowed("legal_entity", "anything"));
}

#[test]
fn data_permission_is_relation_allowed() {
    let mut perm = DataPermission::default();
    perm.allowed_relations = vec!["has_founder".to_string()];
    perm.denied_relations = vec!["has_manager".to_string()];
    assert!(perm.is_relation_allowed("has_founder"));
    assert!(!perm.is_relation_allowed("has_manager"));
    // Not in allow list
    assert!(!perm.is_relation_allowed("has_branch"));
}

#[test]
fn data_rule_is_valid_at() {
    let rule = DataRule {
        rule_id: "r1".to_string(),
        effect: Effect::Allow,
        subject: "g1".to_string(),
        action: "read".to_string(),
        resource: ResourceSelector::default(),
        condition: None,
        fields: vec![],
        relations: vec![],
        mask: None,
        obligations: vec![],
        priority: 10,
        valid_from: chrono::Utc::now() - chrono::Duration::days(1),
        valid_to: Some(chrono::Utc::now() + chrono::Duration::days(1)),
        policy_revision_id: "rev-001".to_string(),
        reason: "test".to_string(),
    };
    assert!(rule.is_valid_at(chrono::Utc::now()));
    assert!(!rule.is_valid_at(chrono::Utc::now() + chrono::Duration::days(2)));
}

#[test]
fn data_rule_is_deny() {
    let mut rule = DataRule {
        rule_id: "r1".to_string(),
        effect: Effect::Allow,
        subject: "g1".to_string(),
        action: "read".to_string(),
        resource: ResourceSelector::default(),
        condition: None,
        fields: vec![],
        relations: vec![],
        mask: None,
        obligations: vec![],
        priority: 10,
        valid_from: chrono::Utc::now(),
        valid_to: None,
        policy_revision_id: "rev-001".to_string(),
        reason: "test".to_string(),
    };
    assert!(!rule.is_deny());
    rule.effect = Effect::Deny;
    assert!(rule.is_deny());
}

#[test]
fn data_rule_matches() {
    let rule = DataRule {
        rule_id: "r1".to_string(),
        effect: Effect::Allow,
        subject: "group-001".to_string(),
        action: "read".to_string(),
        resource: ResourceSelector::default(),
        condition: None,
        fields: vec![],
        relations: vec![],
        mask: None,
        obligations: vec![],
        priority: 10,
        valid_from: chrono::Utc::now(),
        valid_to: None,
        policy_revision_id: "rev-001".to_string(),
        reason: "test".to_string(),
    };
    assert!(rule.matches("group-001", "read"));
    assert!(!rule.matches("group-002", "read"));
    assert!(!rule.matches("group-001", "write"));
}

// --- PDP tests ---

fn make_principal(tenant_id: &str, group_ids: Vec<&str>) -> Principal {
    Principal {
        principal_id: "user-001".to_string(),
        kind: PrincipalKind::User,
        tenant_id: tenant_id.to_string(),
        group_ids: group_ids.into_iter().map(String::from).collect(),
        auth_strength: "mfa".to_string(),
        purpose: None,
    }
}

fn make_session(step_up: bool) -> SessionInfo {
    SessionInfo {
        session_id: "sess-001".to_string(),
        device_id: None,
        ip_address: "127.0.0.1".to_string(),
        user_agent: Some("test".to_string()),
        mfa_verified: true,
        step_up,
        risk_score: "0.0".to_string(),
    }
}

fn make_context(action: &str) -> DecisionContext {
    DecisionContext {
        action: action.to_string(),
        resource_type: None,
        resource_id: None,
        requested_fields: vec![],
        requested_relations: vec![],
        high_risk: false,
    }
}

fn make_snapshot_with_caps(tenant_id: &str, caps: Vec<Capability>, groups: Vec<CapabilityGroup>) -> PolicySnapshot {
    let mut snapshot = PolicySnapshot::new("rev-001", tenant_id, "grev-001");
    for cap in caps {
        snapshot.add_capability(cap);
    }
    for group in groups {
        snapshot.add_group(group);
    }
    snapshot
}

#[test]
fn pdp_deny_tenant_mismatch() {
    let snapshot = PolicySnapshot::new("rev-001", "tenant-A", "grev-001");
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-B", vec![]);
    let session = make_session(false);
    let ctx = make_context("entity.create");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    assert_eq!(decision.kind, DecisionKind::Deny);
    assert!(decision.reason.contains("tenant mismatch"));
}

#[test]
fn pdp_deny_capability_missing() {
    let snapshot = make_snapshot_with_caps(
        "tenant-A",
        vec![Capability {
            name: "entity.create".to_string(),
            kind: CapabilityKind::System,
            description_key: "cap".to_string(),
            requires_step_up: false,
            high_risk: false,
        }],
        vec![CapabilityGroup {
            group_id: "g1".to_string(),
            name: "Editors".to_string(),
            kind: CapabilityGroupKind::System,
            capabilities: vec!["entity.create".to_string()],
            description_key: "group".to_string(),
            owner: "team".to_string(),
        }],
    );
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-A", vec!["g1"]);
    let session = make_session(false);
    // Request a capability the principal doesn't have
    let ctx = make_context("entity.retire");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    assert_eq!(decision.kind, DecisionKind::Deny);
    assert!(decision.reason.contains("capability_missing"));
}

#[test]
fn pdp_allow_with_capability() {
    let snapshot = make_snapshot_with_caps(
        "tenant-A",
        vec![Capability {
            name: "entity.create".to_string(),
            kind: CapabilityKind::System,
            description_key: "cap".to_string(),
            requires_step_up: false,
            high_risk: false,
        }],
        vec![CapabilityGroup {
            group_id: "g1".to_string(),
            name: "Editors".to_string(),
            kind: CapabilityGroupKind::System,
            capabilities: vec!["entity.create".to_string()],
            description_key: "group".to_string(),
            owner: "team".to_string(),
        }],
    );
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-A", vec!["g1"]);
    let session = make_session(false);
    let ctx = make_context("entity.create");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    assert_eq!(decision.kind, DecisionKind::Allow);
    assert!(decision.is_allowed());
}

#[test]
fn pdp_step_up_required() {
    let snapshot = make_snapshot_with_caps(
        "tenant-A",
        vec![Capability {
            name: "report.export".to_string(),
            kind: CapabilityKind::System,
            description_key: "cap".to_string(),
            requires_step_up: true,
            high_risk: true,
        }],
        vec![CapabilityGroup {
            group_id: "g1".to_string(),
            name: "Reporters".to_string(),
            kind: CapabilityGroupKind::System,
            capabilities: vec!["report.export".to_string()],
            description_key: "group".to_string(),
            owner: "team".to_string(),
        }],
    );
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-A", vec!["g1"]);
    let session = make_session(false); // no step-up
    let ctx = make_context("report.export");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    assert_eq!(decision.kind, DecisionKind::StepUpRequired);
}

#[test]
fn pdp_step_up_satisfied() {
    let snapshot = make_snapshot_with_caps(
        "tenant-A",
        vec![Capability {
            name: "report.export".to_string(),
            kind: CapabilityKind::System,
            description_key: "cap".to_string(),
            requires_step_up: true,
            high_risk: true,
        }],
        vec![CapabilityGroup {
            group_id: "g1".to_string(),
            name: "Reporters".to_string(),
            kind: CapabilityGroupKind::System,
            capabilities: vec!["report.export".to_string()],
            description_key: "group".to_string(),
            owner: "team".to_string(),
        }],
    );
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-A", vec!["g1"]);
    let session = make_session(true); // step-up verified
    let ctx = make_context("report.export");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    assert_eq!(decision.kind, DecisionKind::Allow);
}

#[test]
fn pdp_deny_overrides_allow() {
    let mut snapshot = make_snapshot_with_caps(
        "tenant-A",
        vec![Capability {
            name: "entity.read".to_string(),
            kind: CapabilityKind::System,
            description_key: "cap".to_string(),
            requires_step_up: false,
            high_risk: false,
        }],
        vec![CapabilityGroup {
            group_id: "g1".to_string(),
            name: "Readers".to_string(),
            kind: CapabilityGroupKind::System,
            capabilities: vec!["entity.read".to_string()],
            description_key: "group".to_string(),
            owner: "team".to_string(),
        }],
    );
    // Add allow rule
    snapshot.add_data_rule(DataRule {
        rule_id: "allow-001".to_string(),
        effect: Effect::Allow,
        subject: "g1".to_string(),
        action: "entity.read".to_string(),
        resource: ResourceSelector {
            entity_type: Some("legal_entity".to_string()),
            ..Default::default()
        },
        condition: None,
        fields: vec![],
        relations: vec![],
        mask: None,
        obligations: vec![],
        priority: 10,
        valid_from: chrono::Utc::now() - chrono::Duration::days(1),
        valid_to: None,
        policy_revision_id: "rev-001".to_string(),
        reason: "allow readers".to_string(),
    });
    // Add deny rule (higher priority)
    snapshot.add_data_rule(DataRule {
        rule_id: "deny-001".to_string(),
        effect: Effect::Deny,
        subject: "g1".to_string(),
        action: "entity.read".to_string(),
        resource: ResourceSelector {
            entity_type: Some("person".to_string()),
            ..Default::default()
        },
        condition: None,
        fields: vec![],
        relations: vec![],
        mask: None,
        obligations: vec![],
        priority: 20,
        valid_from: chrono::Utc::now() - chrono::Duration::days(1),
        valid_to: None,
        policy_revision_id: "rev-001".to_string(),
        reason: "deny person access".to_string(),
    });
    let pdp = LocalPdpClient::new(snapshot);
    let principal = make_principal("tenant-A", vec!["g1"]);
    let session = make_session(false);
    let ctx = make_context("entity.read");
    let decision = pdp.evaluate(&principal, &session, &ctx).unwrap();
    // Deny overrides allow
    assert_eq!(decision.kind, DecisionKind::Deny);
    assert!(decision.reason.contains("denied by explicit deny rule"));
}

#[test]
fn pdp_policy_snapshot_retrieval() {
    let snapshot = PolicySnapshot::new("rev-001", "tenant-A", "grev-001");
    let pdp = LocalPdpClient::new(snapshot);
    let result = pdp.policy_snapshot("tenant-A");
    assert!(result.is_ok());
    let result = pdp.policy_snapshot("tenant-B");
    assert!(result.is_err());
}

#[test]
fn decision_allow_constructor() {
    let decision = Decision::allow(
        "user-001",
        "tenant-A",
        "entity.read",
        "rev-001",
        vec!["rule-001".to_string()],
        DataPermission::default(),
        vec!["log".to_string()],
    );
    assert!(decision.is_allowed());
    assert!(!decision.is_denied());
    assert_eq!(decision.obligations, vec!["log".to_string()]);
    assert!(decision.data_permission.is_some());
}

#[test]
fn decision_deny_constructor() {
    let decision = Decision::deny(
        "user-001",
        "tenant-A",
        "entity.read",
        "rev-001",
        vec!["rule-001".to_string()],
        "no access",
    );
    assert!(decision.is_denied());
    assert!(!decision.is_allowed());
    assert_eq!(decision.reason, "no access");
}

#[test]
fn decision_break_glass_required() {
    let decision = Decision::break_glass_required(
        "user-001",
        "tenant-A",
        "entity.read",
        "rev-001",
        "break-glass needed",
    );
    assert_eq!(decision.kind, DecisionKind::BreakGlassRequired);
}

#[test]
fn decision_step_up_required() {
    let decision = Decision::step_up_required(
        "user-001",
        "tenant-A",
        "report.export",
        "rev-001",
        "MFA required",
    );
    assert_eq!(decision.kind, DecisionKind::StepUpRequired);
}

// --- Audit tests ---

#[test]
fn audit_entry_new() {
    let entry = AuditEntry::new(1, "0000000000000000", "tenant-A", "user-001", AuditKind::Decision, "entity.read");
    assert_eq!(entry.sequence, 1);
    assert_eq!(entry.tenant_id, "tenant-A");
    assert_eq!(entry.actor, "user-001");
    assert_eq!(entry.kind, AuditKind::Decision);
    assert_eq!(entry.action, "entity.read");
    assert!(entry.hash.is_empty()); // hash computed by chain
}

#[test]
fn audit_entry_with_effective_actor() {
    let entry = AuditEntry::new(1, "0", "tenant-A", "admin-001", AuditKind::Impersonation, "impersonate")
        .with_effective_actor("user-002");
    assert_eq!(entry.effective_actor.as_deref(), Some("user-002"));
}

#[test]
fn audit_entry_with_resource() {
    let entry = AuditEntry::new(1, "0", "tenant-A", "user-001", AuditKind::Decision, "entity.read")
        .with_resource("legal_entity", "ent-001");
    assert_eq!(entry.resource_type.as_deref(), Some("legal_entity"));
    assert_eq!(entry.resource_id.as_deref(), Some("ent-001"));
}

#[test]
fn audit_entry_with_decision() {
    let entry = AuditEntry::new(1, "0", "tenant-A", "user-001", AuditKind::Decision, "entity.read")
        .with_decision("allow", vec!["rule-001".to_string()]);
    assert_eq!(entry.decision.as_deref(), Some("allow"));
    assert_eq!(entry.reason_ids, vec!["rule-001".to_string()]);
}

#[test]
fn audit_entry_with_metrics() {
    let entry = AuditEntry::new(1, "0", "tenant-A", "user-001", AuditKind::Export, "report.export")
        .with_metrics(Some(500), Some(1024 * 1024));
    assert_eq!(entry.row_count, Some(500));
    assert_eq!(entry.bytes, Some(1024 * 1024));
}

#[test]
fn audit_entry_add_metadata() {
    let mut entry = AuditEntry::new(1, "0", "tenant-A", "user-001", AuditKind::Decision, "entity.read");
    entry.add_metadata("source", "api");
    entry.add_metadata("version", "1.0");
    assert_eq!(entry.metadata.len(), 2);
}

#[test]
fn audit_entry_compute_hash() {
    let ts = chrono::Utc::now();
    let mut entry = AuditEntry::new(1, "0000000000000000", "tenant-A", "user-001", AuditKind::Decision, "entity.read");
    entry.timestamp = ts;
    let hash = entry.compute_hash();
    assert!(!hash.is_empty());
    // Same input → same hash
    let mut entry2 = AuditEntry::new(1, "0000000000000000", "tenant-A", "user-001", AuditKind::Decision, "entity.read");
    entry2.timestamp = ts;
    assert_eq!(hash, entry2.compute_hash());
}

#[test]
fn tamper_chain_new() {
    let chain = TamperChain::new();
    assert_eq!(chain.sequence, 0);
    assert_eq!(chain.total_entries, 0);
    assert_eq!(chain.last_hash, "0000000000000000");
}

#[test]
fn tamper_chain_append() {
    let mut chain = TamperChain::new();
    let entry1 = AuditEntry::new(0, "0", "tenant-A", "user-001", AuditKind::Decision, "action1");
    let appended1 = chain.append(entry1);
    assert_eq!(appended1.sequence, 0);
    assert!(!appended1.hash.is_empty());
    assert_eq!(chain.sequence, 1);
    assert_eq!(chain.total_entries, 1);

    let entry2 = AuditEntry::new(0, "0", "tenant-A", "user-001", AuditKind::Decision, "action2");
    let appended2 = chain.append(entry2);
    assert_eq!(appended2.sequence, 1);
    assert_eq!(appended2.previous_hash, appended1.hash);
    assert_eq!(chain.sequence, 2);
}

#[test]
fn tamper_chain_checkpoint() {
    let mut chain = TamperChain::new();
    chain.append(AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, "a"));
    chain.append(AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, "b"));
    chain.checkpoint();
    assert_eq!(chain.checkpoints.len(), 1);
    let (seq, hash) = chain.latest_checkpoint().unwrap();
    assert_eq!(*seq, 2);
    assert_eq!(*hash, chain.last_hash);
}

#[test]
fn tamper_chain_verify_valid() {
    let mut chain = TamperChain::new();
    let mut entries = Vec::new();
    for i in 0..5 {
        let entry = AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, &format!("action{}", i));
        let appended = chain.append(entry);
        entries.push(appended);
    }
    assert!(chain.verify(&entries));
}

#[test]
fn tamper_chain_verify_tampered() {
    let mut chain = TamperChain::new();
    let mut entries = Vec::new();
    for i in 0..3 {
        let entry = AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, &format!("action{}", i));
        let appended = chain.append(entry);
        entries.push(appended);
    }
    // Tamper with an entry
    entries[1].action = "tampered".to_string();
    assert!(!chain.verify(&entries));
}

#[test]
fn audit_retention_default() {
    let retention = AuditRetention::default_policy();
    assert_eq!(retention.retention_days, 365);
    assert!(!retention.legal_hold);
    assert!(retention.archive_before_delete);
}

#[test]
fn audit_retention_is_expired() {
    let retention = AuditRetention::default_policy();
    let old_entry = AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, "a");
    let mut old_entry = old_entry;
    old_entry.timestamp = chrono::Utc::now() - chrono::Duration::days(400);
    assert!(retention.is_expired(&old_entry));
}

#[test]
fn audit_retention_legal_hold_prevents_expiry() {
    let mut retention = AuditRetention::default_policy();
    retention.legal_hold = true;
    let mut old_entry = AuditEntry::new(0, "0", "t", "u", AuditKind::Decision, "a");
    old_entry.timestamp = chrono::Utc::now() - chrono::Duration::days(400);
    assert!(!retention.is_expired(&old_entry));
}

#[test]
fn audit_query_defaults() {
    let query = AuditQuery {
        tenant_id: "tenant-A".to_string(),
        kind: None,
        actor: None,
        action: None,
        resource_type: None,
        resource_id: None,
        decision: None,
        from: None,
        to: None,
        limit: Some(100),
        cursor: None,
    };
    assert_eq!(query.tenant_id, "tenant-A");
    assert_eq!(query.limit, Some(100));
}
