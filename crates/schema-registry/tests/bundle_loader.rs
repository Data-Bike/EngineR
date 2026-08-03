//! SKR bundle loader integration tests (docs/03 §10, docs/24 §9).

#![cfg(test)]

use enginer_domain::TenantId;
use enginer_schema_registry::{LoadedBundle, SchemaDraft};

#[test]
fn load_skr_core_bundle() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    assert_eq!(bundle.bundle_id, "skr-core");
    assert_eq!(bundle.version, "1.0.0");
    // 7 core + 8 domain = 15 types
    assert_eq!(bundle.types.len(), 15, "expected 15 types");
    // ~45 core + ~40 domain = ~85 predicates (exact count may vary)
    assert!(bundle.predicates.len() >= 80, "expected >= 80 predicates");
}

#[test]
fn bundle_has_person_and_legal_entity() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let aliases: Vec<&str> = bundle.types.iter().map(|t| t.stable_alias.as_str()).collect();
    assert!(aliases.contains(&"person"), "missing person");
    assert!(aliases.contains(&"legal_entity"), "missing legal_entity");
}

#[test]
fn bundle_has_request_check_risk() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let aliases: Vec<&str> = bundle.types.iter().map(|t| t.stable_alias.as_str()).collect();
    assert!(aliases.contains(&"request"), "missing request");
    assert!(aliases.contains(&"check"), "missing check");
    assert!(aliases.contains(&"risk_factor"), "missing risk_factor");
    assert!(aliases.contains(&"assessment"), "missing assessment");
}

#[test]
fn bundle_to_schema_draft() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let tenant = TenantId::new();
    let draft = bundle.to_schema_draft(tenant);
    assert_eq!(draft.types.len(), 15);
    assert!(draft.predicates.len() >= 80);
    assert_eq!(draft.tenant_id, tenant);
}

#[test]
fn bundle_predicates_have_valid_scalar_kinds() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    for pred in &bundle.predicates {
        if !pred.is_edge {
            assert!(
                pred.scalar_kind.is_some(),
                "scalar predicate {} missing scalar_kind",
                pred.stable_alias
            );
        }
    }
}

#[test]
fn bundle_edge_predicates_have_range() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    for pred in &bundle.predicates {
        if pred.is_edge {
            assert!(
                pred.range_type_id.is_some(),
                "edge predicate {} missing range_type_id",
                pred.stable_alias
            );
        }
    }
}

#[test]
fn bundle_identity_predicates_are_scalar() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    for pred in &bundle.predicates {
        if pred.participates_in_identity {
            assert!(
                !pred.is_edge,
                "identity predicate {} is an edge, must be scalar",
                pred.stable_alias
            );
        }
    }
}

#[test]
fn bundle_no_duplicate_predicate_ids() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let mut seen = std::collections::HashSet::new();
    for pred in &bundle.predicates {
        assert!(
            seen.insert(pred.predicate_id.0),
            "duplicate predicate_id: {}",
            pred.predicate_id
        );
    }
}

#[test]
fn bundle_no_duplicate_type_ids() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let mut seen = std::collections::HashSet::new();
    for ty in &bundle.types {
        assert!(
            seen.insert(ty.type_id.0),
            "duplicate type_id: {}",
            ty.type_id
        );
    }
}

#[test]
fn bundle_draft_can_be_validated() {
    let dir = enginer_schema_registry::bundle::skr_core_dir();
    let bundle = LoadedBundle::from_dir(&dir).expect("bundle loads");
    let tenant = TenantId::new();
    let draft = bundle.to_schema_draft(tenant);
    // Draft should have types and predicates populated
    assert!(!draft.types.is_empty());
    assert!(!draft.predicates.is_empty());
    // Draft should be in Draft state
    assert_eq!(
        draft.state,
        enginer_domain::schema::SchemaRevisionState::Draft
    );
}
