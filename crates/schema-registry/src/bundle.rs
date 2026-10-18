//! SKR schema bundle loader (docs/03 §10, docs/24 §9).
//!
//! A schema bundle is a signed JSON package loaded via Admin API. The loader
//! parses the bundle files and converts them into a `SchemaDraft` ready for
//! validation and publish (docs/12 §1).
//!
//! The core never imports a crate of the package: the bundle is data, not code.

use enginer_domain::{
    schema::{
        EntityClass, PredicateDefinition, PredicateDirection, TypeDefinition, TypePredicateSlot,
    },
    PredicateId, ScalarKind, TenantId, TypeId,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Error loading a schema bundle.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum BundleLoadError {
    #[error("io: {message}")]
    Io { message: String },
    #[error("json: {message}")]
    Json { message: String },
    #[error("missing_file: {file}")]
    MissingFile { file: String },
    #[error("missing_field: {field} in {context}")]
    MissingField { field: String, context: String },
    #[error("invalid_uuid: {value} in {context}")]
    InvalidUuid { value: String, context: String },
    #[error("invalid_scalar_kind: {value}")]
    InvalidScalarKind { value: String },
    #[error("invalid_entity_class: {value}")]
    InvalidEntityClass { value: String },
    #[error("invalid_direction: {value}")]
    InvalidDirection { value: String },
    #[error("duplicate_id: {id} in {context}")]
    DuplicateId { id: String, context: String },
    #[error("dangling_reference: {reference} ({kind}) in {context}")]
    DanglingReference {
        reference: String,
        kind: String,
        context: String,
    },
}

/// A loaded schema bundle ready to be converted into a `SchemaDraft`.
#[derive(Debug, Clone)]
pub struct LoadedBundle {
    pub bundle_id: String,
    pub version: String,
    pub types: Vec<TypeDefinition>,
    pub predicates: Vec<PredicateDefinition>,
}

impl LoadedBundle {
    /// Load a bundle from a directory containing the JSON files.
    pub fn from_dir(dir: &Path) -> Result<Self, BundleLoadError> {
        let manifest = load_json(dir, "manifest.json")?;
        let bundle_id = manifest
            .get("bundle_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "bundle_id".to_string(),
                context: "manifest.json".to_string(),
            })?
            .to_string();
        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "version".to_string(),
                context: "manifest.json".to_string(),
            })?
            .to_string();

        let mut types = Vec::new();
        let mut predicates = Vec::new();
        let mut seen_type_ids = HashSet::new();
        let mut seen_pred_ids = HashSet::new();

        // Load types.json (core types)
        let types_json = load_json(dir, "types.json")?;
        parse_types(&types_json, &mut types, &mut seen_type_ids, "types.json")?;
        // Predicates for core types are in predicates.json
        let preds_json = load_json(dir, "predicates.json")?;
        parse_predicates(
            &preds_json,
            &mut predicates,
            &mut seen_pred_ids,
            "predicates.json",
        )?;

        // Load domain-types.json (domain types + their predicates inline)
        let domain_json = load_json(dir, "domain-types.json")?;
        parse_types(
            &domain_json,
            &mut types,
            &mut seen_type_ids,
            "domain-types.json",
        )?;
        parse_predicates(
            &domain_json,
            &mut predicates,
            &mut seen_pred_ids,
            "domain-types.json",
        )?;

        // Validate referential integrity
        validate_references(&types, &predicates)?;

        Ok(Self {
            bundle_id,
            version,
            types,
            predicates,
        })
    }

    /// Convert the loaded bundle into a `SchemaDraft` for validation/publish.
    pub fn to_schema_draft(&self, tenant: TenantId) -> crate::draft::SchemaDraft {
        let mut draft = crate::draft::SchemaDraft::new(
            tenant, None, // no parent revision
        );
        draft.semantic_version = self.version.clone();
        for ty in &self.types {
            // add_type clones, so this is safe
            draft
                .add_type(ty.clone())
                .expect("bundle types are pre-validated");
        }
        for pred in &self.predicates {
            draft
                .add_predicate(pred.clone())
                .expect("bundle predicates are pre-validated");
        }
        draft
    }
}

fn load_json(dir: &Path, name: &str) -> Result<serde_json::Value, BundleLoadError> {
    let path = dir.join(name);
    let content = std::fs::read_to_string(&path).map_err(|e| BundleLoadError::Io {
        message: format!("failed to read {}: {}", path.display(), e),
    })?;
    serde_json::from_str(&content).map_err(|e| BundleLoadError::Json {
        message: format!("failed to parse {}: {}", name, e),
    })
}

fn parse_uuid(s: &str, context: &str) -> Result<uuid::Uuid, BundleLoadError> {
    uuid::Uuid::parse_str(s).map_err(|_| BundleLoadError::InvalidUuid {
        value: s.to_string(),
        context: context.to_string(),
    })
}

fn parse_scalar_kind(s: &str) -> Result<ScalarKind, BundleLoadError> {
    match s {
        "text" => Ok(ScalarKind::Text),
        "localized_text" => Ok(ScalarKind::LocalizedText),
        "boolean" => Ok(ScalarKind::Boolean),
        "integer" => Ok(ScalarKind::Integer),
        "decimal" => Ok(ScalarKind::Decimal),
        "money" => Ok(ScalarKind::Money),
        "date" => Ok(ScalarKind::Date),
        "datetime" | "instant" => Ok(ScalarKind::Instant),
        "local_datetime" => Ok(ScalarKind::LocalDateTime),
        "uuid" => Ok(ScalarKind::Uuid),
        "uri" => Ok(ScalarKind::Uri),
        "email" => Ok(ScalarKind::Email),
        "phone" => Ok(ScalarKind::Phone),
        "measurement" => Ok(ScalarKind::Measurement),
        "geo" => Ok(ScalarKind::Geo),
        "json" => Ok(ScalarKind::Json),
        "blob" => Ok(ScalarKind::Blob),
        other => Err(BundleLoadError::InvalidScalarKind {
            value: other.to_string(),
        }),
    }
}

fn parse_entity_class(s: &str) -> Result<EntityClass, BundleLoadError> {
    match s {
        "data" => Ok(EntityClass::Data),
        "schema" => Ok(EntityClass::Schema),
        "policy" => Ok(EntityClass::Policy),
        "workflow" => Ok(EntityClass::Workflow),
        "system" => Ok(EntityClass::System),
        other => Err(BundleLoadError::InvalidEntityClass {
            value: other.to_string(),
        }),
    }
}

fn parse_direction(s: &str) -> Result<PredicateDirection, BundleLoadError> {
    match s {
        "directed" => Ok(PredicateDirection::Directed),
        "symmetric" => Ok(PredicateDirection::Symmetric),
        other => Err(BundleLoadError::InvalidDirection {
            value: other.to_string(),
        }),
    }
}

fn parse_types(
    v: &serde_json::Value,
    types: &mut Vec<TypeDefinition>,
    seen_ids: &mut HashSet<uuid::Uuid>,
    context: &str,
) -> Result<(), BundleLoadError> {
    let arr =
        v.get("types")
            .and_then(|t| t.as_array())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "types".to_string(),
                context: context.to_string(),
            })?;

    for ty in arr {
        let type_id_raw = ty.get("type_id").and_then(|v| v.as_str()).ok_or_else(|| {
            BundleLoadError::MissingField {
                field: "type_id".to_string(),
                context: format!("{} type", context),
            }
        })?;
        let type_uuid = parse_uuid(type_id_raw, context)?;
        if !seen_ids.insert(type_uuid) {
            return Err(BundleLoadError::DuplicateId {
                id: type_id_raw.to_string(),
                context: context.to_string(),
            });
        }
        let stable_alias = ty
            .get("stable_alias")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "stable_alias".to_string(),
                context: format!("{} type", context),
            })?
            .to_string();
        let entity_class =
            parse_entity_class(ty.get("entity_class").and_then(|v| v.as_str()).ok_or_else(
                || BundleLoadError::MissingField {
                    field: "entity_class".to_string(),
                    context: format!("{} type {}", context, stable_alias),
                },
            )?)?;
        let predicate_slots = ty
            .get("predicates")
            .and_then(|p| p.as_array())
            .map(|slots| {
                slots
                    .iter()
                    .map(|slot| {
                        let pid = slot
                            .get("predicate_id")
                            .and_then(|v| v.as_str())
                            .ok_or_else(|| BundleLoadError::MissingField {
                                field: "predicate_id".to_string(),
                                context: format!("{} type {}", context, stable_alias),
                            })?;
                        Ok(TypePredicateSlot {
                            predicate_id: PredicateId(parse_uuid(pid, context)?),
                            required: slot
                                .get("required")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false),
                            min_cardinality: slot
                                .get("min_cardinality")
                                .and_then(|v| v.as_u64())
                                .unwrap_or(0) as u32,
                            max_cardinality: slot
                                .get("max_cardinality")
                                .and_then(|v| v.as_u64())
                                .map(|n| n as u32),
                            order: slot.get("order").and_then(|v| v.as_u64()).map(|n| n as u32),
                            group: slot.get("group").and_then(|v| v.as_str()).map(String::from),
                        })
                    })
                    .collect::<Result<Vec<_>, BundleLoadError>>()
            })
            .transpose()?
            .unwrap_or_default();
        let identity_policy_ids = ty
            .get("identity_policy_ids")
            .and_then(|v| v.as_array())
            .map(|ids| {
                ids.iter()
                    .filter_map(|id| id.as_str().and_then(|s| uuid::Uuid::parse_str(s).ok()))
                    .collect()
            })
            .unwrap_or_default();

        types.push(TypeDefinition {
            type_id: TypeId(type_uuid),
            stable_alias,
            entity_class,
            predicates: predicate_slots,
            identity_policy_ids,
        });
    }
    Ok(())
}

fn parse_predicates(
    v: &serde_json::Value,
    predicates: &mut Vec<PredicateDefinition>,
    seen_ids: &mut HashSet<uuid::Uuid>,
    context: &str,
) -> Result<(), BundleLoadError> {
    let arr = match v.get("predicates").and_then(|p| p.as_array()) {
        Some(a) => a,
        None => return Ok(()), // predicates may be absent in types.json
    };

    for pred in arr {
        let pid_raw = pred
            .get("predicate_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "predicate_id".to_string(),
                context: format!("{} predicate", context),
            })?;
        let pid_uuid = parse_uuid(pid_raw, context)?;
        if !seen_ids.insert(pid_uuid) {
            return Err(BundleLoadError::DuplicateId {
                id: pid_raw.to_string(),
                context: context.to_string(),
            });
        }
        let stable_alias = pred
            .get("stable_alias")
            .and_then(|v| v.as_str())
            .ok_or_else(|| BundleLoadError::MissingField {
                field: "stable_alias".to_string(),
                context: format!("{} predicate", context),
            })?
            .to_string();
        let is_edge = pred
            .get("is_edge")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let direction = parse_direction(
            pred.get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("directed"),
        )?;
        let domain_type_id = pred
            .get("domain_type_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .map(TypeId);
        let range_type_id = pred
            .get("range_type_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .and_then(|s| uuid::Uuid::parse_str(s).ok())
            .map(TypeId);
        let scalar_kind = pred
            .get("scalar_kind")
            .and_then(|v| v.as_str())
            .map(parse_scalar_kind)
            .transpose()?;
        let min_cardinality = pred
            .get("min_cardinality")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
        let max_cardinality = pred
            .get("max_cardinality")
            .and_then(|v| v.as_u64())
            .map(|n| n as u32);
        let participates_in_identity = pred
            .get("participates_in_identity")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let confidentiality_class = pred
            .get("confidentiality_class")
            .and_then(|v| v.as_str())
            .unwrap_or("public")
            .to_string();
        let normalizer_version = pred
            .get("normalizer_version")
            .and_then(|v| v.as_str())
            .map(String::from);

        predicates.push(PredicateDefinition {
            predicate_id: PredicateId(pid_uuid),
            stable_alias,
            is_edge,
            direction,
            domain_type_id,
            range_type_id,
            scalar_kind,
            min_cardinality,
            max_cardinality,
            participates_in_identity,
            confidentiality_class,
            normalizer_version,
        });
    }
    Ok(())
}

fn validate_references(
    types: &[TypeDefinition],
    predicates: &[PredicateDefinition],
) -> Result<(), BundleLoadError> {
    let type_ids: HashSet<uuid::Uuid> = types.iter().map(|t| t.type_id.0).collect();
    let pred_ids: HashSet<uuid::Uuid> = predicates.iter().map(|p| p.predicate_id.0).collect();

    // Check predicate domain/range references
    for pred in predicates {
        if let Some(domain) = pred.domain_type_id {
            if !type_ids.contains(&domain.0) {
                return Err(BundleLoadError::DanglingReference {
                    reference: domain.to_string(),
                    kind: "domain_type_id".to_string(),
                    context: format!("predicate {}", pred.stable_alias),
                });
            }
        }
        if let Some(range) = pred.range_type_id {
            if !type_ids.contains(&range.0) {
                return Err(BundleLoadError::DanglingReference {
                    reference: range.to_string(),
                    kind: "range_type_id".to_string(),
                    context: format!("predicate {}", pred.stable_alias),
                });
            }
        }
    }

    // Check type predicate slot references
    for ty in types {
        for slot in &ty.predicates {
            if !pred_ids.contains(&slot.predicate_id.0) {
                return Err(BundleLoadError::DanglingReference {
                    reference: slot.predicate_id.to_string(),
                    kind: "predicate_id".to_string(),
                    context: format!("type {}", ty.stable_alias),
                });
            }
        }
    }

    Ok(())
}

/// Locate the SKR core bundle directory relative to the crate.
pub fn skr_core_dir() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("..");
    p.push("..");
    p.push("schema-packages");
    p.push("skr-core");
    p
}
