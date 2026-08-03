//! SKR schema bundle tests (docs/03 §10, docs/24 §9).
//!
//! The SKR core bundle is a signed JSON package loaded via Admin API.
//! These tests validate structural integrity, referential consistency
//! and invariant compliance of the bundle files.

#![cfg(test)]

use std::collections::HashSet;
use std::path::PathBuf;

/// SKR bundle file location.
fn bundle_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("..");
    p.push("schema-packages");
    p.push("skr-core");
    p
}

fn load_json(name: &str) -> serde_json::Value {
    let path = bundle_dir().join(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("failed to parse {}: {}", path.display(), e))
}

/// Collect all type_ids from a types array.
fn collect_type_ids(types: &[serde_json::Value]) -> HashSet<String> {
    types
        .iter()
        .filter_map(|t| t.get("type_id").and_then(|v| v.as_str()).map(String::from))
        .collect()
}

/// Collect all predicate_ids from a predicates array.
fn collect_predicate_ids(preds: &[serde_json::Value]) -> HashSet<String> {
    preds
        .iter()
        .filter_map(|p| p.get("predicate_id").and_then(|v| v.as_str()).map(String::from))
        .collect()
}

/// Helper: get predicates array from a JSON value, or empty slice.
fn predicates_array(v: &serde_json::Value) -> &[serde_json::Value] {
    v.get("predicates").and_then(|p| p.as_array()).map_or(&[], |v| v)
}

/// Helper: get types array from a JSON value, or empty slice.
fn types_array(v: &serde_json::Value) -> &[serde_json::Value] {
    v.get("types").and_then(|t| t.as_array()).map_or(&[], |v| v)
}

#[test]
fn types_json_loads_and_has_core_types() {
    let v = load_json("types.json");
    let types = types_array(&v);
    assert!(!types.is_empty(), "types.json has no types");
    let aliases: Vec<&str> = types
        .iter()
        .filter_map(|t| t.get("stable_alias").and_then(|a| a.as_str()))
        .collect();
    assert!(aliases.contains(&"person"), "missing person type");
    assert!(aliases.contains(&"legal_entity"), "missing legal_entity type");
    assert!(aliases.contains(&"holding_view"), "missing holding_view type");
    assert!(aliases.contains(&"address"), "missing address type");
    assert!(aliases.contains(&"contact_point"), "missing contact_point type");
    assert!(aliases.contains(&"document"), "missing document type");
    assert!(aliases.contains(&"license"), "missing license type");
}

#[test]
fn domain_types_json_loads_and_has_domain_types() {
    let v = load_json("domain-types.json");
    let types = types_array(&v);
    assert!(!types.is_empty(), "domain-types.json has no types");
    let aliases: Vec<&str> = types
        .iter()
        .filter_map(|t| t.get("stable_alias").and_then(|a| a.as_str()))
        .collect();
    assert!(aliases.contains(&"request"), "missing request type");
    assert!(aliases.contains(&"check"), "missing check type");
    assert!(aliases.contains(&"risk_factor"), "missing risk_factor type");
    assert!(aliases.contains(&"assessment"), "missing assessment type");
    assert!(aliases.contains(&"decision"), "missing decision type");
    assert!(aliases.contains(&"task"), "missing task type");
    assert!(aliases.contains(&"contract"), "missing contract type");
    assert!(aliases.contains(&"collateral"), "missing collateral type");
}

#[test]
fn no_duplicate_type_ids_across_both_files() {
    let core = load_json("types.json");
    let domain = load_json("domain-types.json");
    let core_ids = collect_type_ids(types_array(&core));
    let domain_ids = collect_type_ids(types_array(&domain));
    let all_ids: HashSet<_> = core_ids.iter().chain(domain_ids.iter()).cloned().collect();
    assert_eq!(
        all_ids.len(),
        core_ids.len() + domain_ids.len(),
        "duplicate type_ids across types.json and domain-types.json"
    );
}

#[test]
fn no_duplicate_predicate_ids_across_both_files() {
    let core = load_json("types.json");
    let domain = load_json("domain-types.json");
    let core_preds: HashSet<String> = collect_predicate_ids(predicates_array(&core));
    let domain_preds = collect_predicate_ids(predicates_array(&domain));
    let all_preds: HashSet<_> = core_preds.iter().chain(domain_preds.iter()).cloned().collect();
    assert_eq!(
        all_preds.len(),
        core_preds.len() + domain_preds.len(),
        "duplicate predicate_ids across types.json and domain-types.json"
    );
}

#[test]
fn predicate_domain_range_references_valid_types() {
    let core = load_json("types.json");
    let predicates = load_json("predicates.json");
    let domain = load_json("domain-types.json");
    let core_types = collect_type_ids(types_array(&core));
    let domain_types = collect_type_ids(types_array(&domain));
    let all_types: HashSet<&str> = core_types
        .iter()
        .chain(domain_types.iter())
        .map(|s| s.as_str())
        .collect();

    for preds in [predicates_array(&predicates), predicates_array(&domain)] {
        for pred in preds {
            if let Some(domain_id) = pred.get("domain_type_id").and_then(|v| v.as_str()) {
                if !domain_id.is_empty() {
                    assert!(
                        all_types.contains(domain_id),
                        "predicate {} references unknown domain_type_id {}",
                        pred["stable_alias"].as_str().unwrap_or("?"),
                        domain_id
                    );
                }
            }
            if let Some(range_id) = pred.get("range_type_id").and_then(|v| v.as_str()) {
                if !range_id.is_empty() {
                    assert!(
                        all_types.contains(range_id),
                        "predicate {} references unknown range_type_id {}",
                        pred["stable_alias"].as_str().unwrap_or("?"),
                        range_id
                    );
                }
            }
        }
    }
}

#[test]
fn type_predicate_slots_reference_valid_predicates() {
    let core = load_json("types.json");
    let predicates = load_json("predicates.json");
    let domain = load_json("domain-types.json");
    // Predicates are split across predicates.json and domain-types.json
    let core_preds = collect_predicate_ids(predicates_array(&predicates));
    let domain_preds = collect_predicate_ids(predicates_array(&domain));
    let all_preds: HashSet<&str> = core_preds
        .iter()
        .chain(domain_preds.iter())
        .map(|s| s.as_str())
        .collect();

    for types in [types_array(&core), types_array(&domain)] {
        for ty in types {
            let alias = ty["stable_alias"].as_str().unwrap_or("?");
            if let Some(preds) = ty.get("predicates").and_then(|p| p.as_array()) {
                for slot in preds {
                    let pid = slot["predicate_id"]
                        .as_str()
                        .unwrap_or_else(|| panic!("type {} has slot without predicate_id", alias));
                    assert!(
                        all_preds.contains(pid),
                        "type {} references unknown predicate_id {}",
                        alias,
                        pid
                    );
                }
            }
        }
    }
}

#[test]
fn edge_predicates_have_range_type() {
    let predicates = load_json("predicates.json");
    let domain = load_json("domain-types.json");
    for preds in [predicates_array(&predicates), predicates_array(&domain)] {
        for pred in preds {
            let is_edge = pred.get("is_edge").and_then(|v| v.as_bool()).unwrap_or(false);
            if is_edge {
                let range = pred.get("range_type_id").and_then(|v| v.as_str());
                assert!(
                    range.is_some() && !range.unwrap().is_empty(),
                    "edge predicate {} missing range_type_id",
                    pred["stable_alias"].as_str().unwrap_or("?")
                );
            }
        }
    }
}

#[test]
fn scalar_predicates_have_scalar_kind() {
    let predicates = load_json("predicates.json");
    let domain = load_json("domain-types.json");
    for preds in [predicates_array(&predicates), predicates_array(&domain)] {
        for pred in preds {
            let is_edge = pred.get("is_edge").and_then(|v| v.as_bool()).unwrap_or(false);
            if !is_edge {
                let kind = pred.get("scalar_kind").and_then(|v| v.as_str());
                assert!(
                    kind.is_some(),
                    "scalar predicate {} missing scalar_kind",
                    pred["stable_alias"].as_str().unwrap_or("?")
                );
            }
        }
    }
}

#[test]
fn identity_predicates_are_scalar() {
    let predicates = load_json("predicates.json");
    let domain = load_json("domain-types.json");
    for preds in [predicates_array(&predicates), predicates_array(&domain)] {
        for pred in preds {
            let participates = pred
                .get("participates_in_identity")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if participates {
                let is_edge = pred.get("is_edge").and_then(|v| v.as_bool()).unwrap_or(false);
                assert!(
                    !is_edge,
                    "identity predicate {} is an edge, must be scalar",
                    pred["stable_alias"].as_str().unwrap_or("?")
                );
            }
        }
    }
}
