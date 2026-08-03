//! Cross-crate invariant tests (docs/05 §3, docs/08, docs/09, docs/04 §13).
//!
//! These tests encode the architectural invariants that must hold across the
//! whole workspace, not just within one crate. They are the executable form of
//! the design rules and are run by `cargo test -p enginer-test-support`.

#![forbid(unsafe_code)]

use enginer_domain::{
    ids::{GroupId, PolicyId, PrincipalId, TenantId},
    schema::{EntityClass, PredicateDefinition, PredicateDirection, TypeDefinition},
    DomainError, PredicateId, SchemaRevisionState, TypeId,
};
use enginer_identity::{normalize_email, normalize_phone, normalize_text, normalize_text_search_key};
use enginer_schema_registry::{SchemaDraft, ValidationReport};
use enginer_security_domain::{
    capabilities::CapabilityAction,
    decision::DecisionEffect,
    policy::{PolicyRule, PolicyRuleEffect, ResourceSelector},
    principal::{GroupClass, PrincipalGroupMembership},
};
use enginer_security_api::{
    client::{LocalSecurityClient, SecurityClient, SecurityClientError, SnapshotProvider},
    contract::{
        ActionDecisionRequest, QueryDecisionRequest, SecurityContextDto,
    },
};
use enginer_security_pdp::PdpSnapshot;
use chrono::{Duration, Utc};
use std::sync::Arc;

/// Invariant: normalization is idempotent (docs/04 §13).
#[test]
fn normalization_is_idempotent() {
    let cases = [
        "  ООО Пример  ",
        "+7 (495) 000-00-00",
        "User@Example.COM",
        "Türkiye",
        "مرحبا",
        "你好",
    ];
    for raw in cases {
        // text
        let a = normalize_text(raw).unwrap();
        let b = normalize_text(&a.value_nfc).unwrap();
        assert_eq!(a, b, "text not idempotent for {raw:?}");
        // search key
        let ka = normalize_text_search_key(raw).unwrap();
        let kb = normalize_text_search_key(&ka.value).unwrap();
        assert_eq!(ka, kb, "search key not idempotent for {raw:?}");
    }
    // phone
    let pa = normalize_phone("+7 (495) 000-00-00").unwrap();
    let pb = normalize_phone(&pa.e164).unwrap();
    assert_eq!(pa, pb);
    // email
    let ea = normalize_email("User@Example.COM").unwrap();
    let eb = normalize_email(&format!("{}@{}", ea.local, ea.domain)).unwrap();
    assert_eq!(ea, eb);
}

/// Invariant: default deny (ADR-011). No rule -> Deny.
#[test]
fn security_default_deny() {
    let snap = PdpSnapshot::empty(1);
    let req = ActionDecisionRequest {
        context: ctx(),
        action: CapabilityAction::EntityCreate.as_str().to_string(),
        resource_kind: "entity".to_string(),
        resource_id: None,
        parameters: serde_json::Map::new(),
    };
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let dec = client.action_decision(req).unwrap();
    assert_eq!(dec.effect, enginer_security_api::contract::DecisionEffectDto::Deny);
}

/// Invariant: deny overrides allow (ADR-011).
#[test]
fn security_deny_overrides_allow() {
    let tenant = TenantId::new();
    let principal = PrincipalId::new();
    let group = GroupId::new();
    let snap = PdpSnapshot {
        policy_revision: 1,
        memberships: vec![membership(principal, group, GroupClass::Capability)],
        rules: vec![
            rule(tenant, group, PolicyRuleEffect::Allow, CapabilityAction::EntityCreate.as_str()),
            rule(tenant, group, PolicyRuleEffect::Deny, CapabilityAction::EntityCreate.as_str()),
        ],
    };
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let req = ActionDecisionRequest {
        context: ctx_with(tenant, principal),
        action: CapabilityAction::EntityCreate.as_str().to_string(),
        resource_kind: "entity".to_string(),
        resource_id: None,
        parameters: serde_json::Map::new(),
    };
    let dec = client.action_decision(req).unwrap();
    assert_eq!(dec.effect, enginer_security_api::contract::DecisionEffectDto::Deny);
}

/// Invariant: capability groups grant capabilities; data groups do NOT
/// (ADR-010). The two classes never mix.
#[test]
fn data_group_does_not_grant_capability() {
    let tenant = TenantId::new();
    let principal = PrincipalId::new();
    let data_group = GroupId::new();
    let snap = PdpSnapshot {
        policy_revision: 1,
        memberships: vec![membership(principal, data_group, GroupClass::Data)],
        rules: vec![rule(tenant, data_group, PolicyRuleEffect::Allow, CapabilityAction::EntityCreate.as_str())],
    };
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let req = ActionDecisionRequest {
        context: ctx_with(tenant, principal),
        action: CapabilityAction::EntityCreate.as_str().to_string(),
        resource_kind: "entity".to_string(),
        resource_id: None,
        parameters: serde_json::Map::new(),
    };
    let dec = client.action_decision(req).unwrap();
    assert_eq!(dec.effect, enginer_security_api::contract::DecisionEffectDto::Deny);
}

/// Invariant: a decision is bound to tenant/principal/session/auth/action/
/// resource and is NOT a bearer permission (docs/09 §9). We verify the wire
/// contract carries the binding fields and the response carries an expiry.
#[test]
fn decision_is_bound_and_expires() {
    let snap = PdpSnapshot::empty(1);
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let mut c = ctx();
    c.auth_strength = "single".to_string();
    let req = ActionDecisionRequest {
        context: c.clone(),
        action: CapabilityAction::EntityCreate.as_str().to_string(),
        resource_kind: "entity".to_string(),
        resource_id: Some(uuid::Uuid::now_v7()),
        parameters: serde_json::Map::new(),
    };
    let dec = client.action_decision(req).unwrap();
    assert!(dec.expires_at > Utc::now());
    // decision_id is a fresh UUID, not a reusable token: a second request with
    // the same context returns a different decision_id (decisions are per-request).
    let req2 = ActionDecisionRequest {
        context: c,
        action: CapabilityAction::EntityCreate.as_str().to_string(),
        resource_kind: "entity".to_string(),
        resource_id: Some(uuid::Uuid::now_v7()),
        parameters: serde_json::Map::new(),
    };
    let dec2 = client.action_decision(req2).unwrap();
    assert_ne!(dec.decision_id, dec2.decision_id);
}

/// Invariant: schema revisions are immutable once published (ADR-004).
/// Re-publishing a draft after publish is rejected.
#[test]
fn schema_revision_immutable_after_publish() {
    let tenant = TenantId::new();
    let mut d = SchemaDraft::new(tenant, None);
    d.add_type(TypeDefinition {
        type_id: TypeId::new(),
        stable_alias: "person".to_string(),
        entity_class: EntityClass::Data,
        predicates: Vec::new(),
        identity_policy_ids: Vec::new(),
    })
    .unwrap();
    d.freeze_for_validation().unwrap();
    let rev = d.publish().unwrap();
    assert_eq!(rev.state, SchemaRevisionState::Published);
    // Mutating after publish fails.
    let res = d.add_type(TypeDefinition {
        type_id: TypeId::new(),
        stable_alias: "org".to_string(),
        entity_class: EntityClass::Data,
        predicates: Vec::new(),
        identity_policy_ids: Vec::new(),
    });
    assert!(res.is_err());
    // Publishing again from Published fails.
    assert!(d.publish().is_err());
}

/// Invariant: schema validation rejects predicates whose domain/range types
/// are not defined (docs/12 §2).
#[test]
fn schema_validation_rejects_dangling_range() {
    let tenant = TenantId::new();
    let mut d = SchemaDraft::new(tenant, None);
    d.add_predicate(PredicateDefinition {
        predicate_id: PredicateId::new(),
        stable_alias: "managed_by".to_string(),
        is_edge: true,
        direction: PredicateDirection::Directed,
        domain_type_id: None,
        range_type_id: Some(TypeId::new()), // not defined
        scalar_kind: None,
        min_cardinality: 0,
        max_cardinality: None,
        participates_in_identity: false,
        confidentiality_class: "internal".to_string(),
        normalizer_version: None,
    })
    .unwrap();
    assert!(d.detect_type_cycles().is_err());
}

/// Invariant: validation report passes only when no error-severity findings.
#[test]
fn validation_report_passes_only_without_errors() {
    let mut r = ValidationReport::empty();
    assert!(r.passes());
    r.add(enginer_schema_registry::Severity::Warning, "W1", "warn", None);
    assert!(r.passes());
    r.add(enginer_schema_registry::Severity::Error, "E1", "err", None);
    assert!(!r.passes());
}

/// Invariant: domain errors are categorized (docs/05 §8).
#[test]
fn domain_errors_categorized() {
    let nf = DomainError::not_found("x");
    assert!(matches!(nf, DomainError::NotFound { .. }));
    let inv = DomainError::invariant("x");
    assert!(matches!(inv, DomainError::Invariant { .. }));
}

/// Invariant: query decision with aggregate operations requires
/// `query.aggregate` capability, not just `query.advanced`.
#[test]
fn query_aggregate_requires_aggregate_capability() {
    let tenant = TenantId::new();
    let principal = PrincipalId::new();
    let group = GroupId::new();
    // Grant query.advanced only.
    let snap = PdpSnapshot {
        policy_revision: 1,
        memberships: vec![membership(principal, group, GroupClass::Capability)],
        rules: vec![rule(tenant, group, PolicyRuleEffect::Allow, CapabilityAction::QueryAdvanced.as_str())],
    };
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let req = QueryDecisionRequest {
        context: ctx_with(tenant, principal),
        action: "query.advanced".to_string(),
        root_type_ids: vec![TypeId::new().raw()],
        selected_predicate_ids: vec![],
        relation_predicate_ids: vec![],
        operations: vec!["aggregate".to_string()],
        requested_page_size: 50,
        requested_graph_depth: 2,
        query_fingerprint: "fp".to_string(),
    };
    let dec = client.query_decision(req).unwrap();
    // aggregate op requires query.aggregate, which is not granted -> deny.
    assert_eq!(dec.effect, enginer_security_api::contract::DecisionEffectDto::Deny);
}

/// Invariant: unknown capability string is rejected (fail closed).
#[test]
fn unknown_capability_rejected() {
    let snap = PdpSnapshot::empty(1);
    let client = LocalSecurityClient::new(Arc::new(StaticSnap(snap)));
    let req = ActionDecisionRequest {
        context: ctx(),
        action: "no.such.capability".to_string(),
        resource_kind: "entity".to_string(),
        resource_id: None,
        parameters: serde_json::Map::new(),
    };
    let res = client.action_decision(req);
    assert!(matches!(
        res,
        Err(SecurityClientError::ContextInvalid { .. })
    ));
}

// --- helpers ----------------------------------------------------------------

fn ctx() -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: TenantId::new().raw(),
        principal_id: PrincipalId::new().raw(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: "test".to_string(),
        locale: "en-US".to_string(),
        time: Utc::now(),
        client_service: "enginer-test".to_string(),
    }
}

fn ctx_with(tenant: TenantId, principal: PrincipalId) -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: tenant.raw(),
        principal_id: principal.raw(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: "test".to_string(),
        locale: "en-US".to_string(),
        time: Utc::now(),
        client_service: "enginer-test".to_string(),
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

fn rule(tenant: TenantId, group: GroupId, effect: PolicyRuleEffect, action: &str) -> PolicyRule {
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
        policy_revision: 1,
        reason: None,
    }
}

/// Snapshot provider that returns a fixed snapshot, ignoring inputs.
struct StaticSnap(PdpSnapshot);

impl SnapshotProvider for StaticSnap {
    fn snapshot(
        &self,
        _tenant: TenantId,
        _principal: PrincipalId,
        _policy_revision: Option<i64>,
    ) -> Result<PdpSnapshot, SecurityClientError> {
        Ok(self.0.clone())
    }
}

// Re-export so the test file compiles even if some imports are flagged unused.
#[allow(dead_code)]
fn _unused() {
    let _ = DecisionEffect::Deny;
}
