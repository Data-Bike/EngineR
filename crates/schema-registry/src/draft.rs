//! Schema draft (docs/12 §1).
//!
//! A draft is a mutable working copy. On publish it is frozen (content hash
//! computed) and becomes an immutable `SchemaRevision`.

use enginer_domain::{
    ContentHash, DomainError, SchemaRevision, SchemaRevisionId, SchemaRevisionState, TenantId,
};
use std::collections::HashSet;

/// Error operating on a draft.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SchemaDraftError {
    #[error("state_invalid: {message}")]
    StateInvalid { message: String },
    #[error("validation: {message}")]
    Validation { message: String },
    #[error("domain: {0}")]
    Domain(enginer_domain::DomainError),
}

impl From<DomainError> for SchemaDraftError {
    fn from(e: DomainError) -> Self {
        Self::Domain(e)
    }
}

/// A mutable schema draft.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDraft {
    pub revision_id: SchemaRevisionId,
    pub tenant_id: TenantId,
    pub state: SchemaRevisionState,
    pub parent_revision_id: Option<SchemaRevisionId>,
    pub semantic_version: String,
    pub types: Vec<enginer_domain::schema::TypeDefinition>,
    pub predicates: Vec<enginer_domain::schema::PredicateDefinition>,
    pub content_hash: Option<ContentHash>,
}

impl SchemaDraft {
    /// Start a new draft for a tenant, optionally based on a parent revision.
    pub fn new(tenant: TenantId, parent: Option<SchemaRevisionId>) -> Self {
        Self {
            revision_id: SchemaRevisionId::new(),
            tenant_id: tenant,
            state: SchemaRevisionState::Draft,
            parent_revision_id: parent,
            semantic_version: "0.1.0".to_string(),
            types: Vec::new(),
            predicates: Vec::new(),
            content_hash: None,
        }
    }

    /// Add a type definition.
    pub fn add_type(&mut self, ty: enginer_domain::schema::TypeDefinition) -> Result<(), SchemaDraftError> {
        if self.state != SchemaRevisionState::Draft {
            return Err(SchemaDraftError::StateInvalid {
                message: "draft is not editable".to_string(),
            });
        }
        if self.types.iter().any(|t| t.type_id == ty.type_id) {
            return Err(SchemaDraftError::Validation {
                message: format!("duplicate type_id {}", ty.type_id),
            });
        }
        self.types.push(ty);
        Ok(())
    }

    /// Add a predicate definition.
    pub fn add_predicate(
        &mut self,
        pred: enginer_domain::schema::PredicateDefinition,
    ) -> Result<(), SchemaDraftError> {
        if self.state != SchemaRevisionState::Draft {
            return Err(SchemaDraftError::StateInvalid {
                message: "draft is not editable".to_string(),
            });
        }
        if self.predicates.iter().any(|p| p.predicate_id == pred.predicate_id) {
            return Err(SchemaDraftError::Validation {
                message: format!("duplicate predicate_id {}", pred.predicate_id),
            });
        }
        self.predicates.push(pred);
        Ok(())
    }

    /// Move to `Validating` (freeze for validation).
    pub fn freeze_for_validation(&mut self) -> Result<(), SchemaDraftError> {
        if self.state != SchemaRevisionState::Draft {
            return Err(SchemaDraftError::StateInvalid {
                message: format!("cannot freeze from {:?}", self.state),
            });
        }
        self.state = SchemaRevisionState::Validating;
        Ok(())
    }

    /// Publish: freeze content hash, transition to `Published`.
    ///
    /// In production this is gated by four-eyes approval, impact report and
    /// `If-Match` (docs/06 §5, docs/12 §1). The registry here performs the
    /// state transition; the approval gate is enforced by the application layer.
    pub fn publish(&mut self) -> Result<SchemaRevision, SchemaDraftError> {
        if !matches!(
            self.state,
            SchemaRevisionState::Validating | SchemaRevisionState::ImpactReady | SchemaRevisionState::Approved
        ) {
            return Err(SchemaDraftError::StateInvalid {
                message: format!("cannot publish from {:?}", self.state),
            });
        }
        let hash = compute_content_hash(&self.types, &self.predicates);
        self.content_hash = Some(hash);
        self.state = SchemaRevisionState::Published;
        Ok(SchemaRevision {
            revision_id: self.revision_id,
            tenant_id: self.tenant_id,
            state: SchemaRevisionState::Published,
            parent_revision_id: self.parent_revision_id,
            semantic_version: self.semantic_version.clone(),
            content_hash: hash,
            types: self.types.clone(),
            predicates: self.predicates.clone(),
        })
    }

    /// Detect inheritance/membership cycles among types (docs/12 §2).
    pub fn detect_type_cycles(&self) -> Result<(), SchemaDraftError> {
        // Phase 0+1: types reference predicates, not other types directly for
        // inheritance. We still verify predicate domain/range types exist.
        let known: HashSet<_> = self.types.iter().map(|t| t.type_id).collect();
        for p in &self.predicates {
            if let Some(dt) = p.domain_type_id {
                if !known.contains(&dt) {
                    return Err(SchemaDraftError::Validation {
                        message: format!("predicate {} domain type {} not defined", p.predicate_id, dt),
                    });
                }
            }
            if let Some(rt) = p.range_type_id {
                if !known.contains(&rt) {
                    return Err(SchemaDraftError::Validation {
                        message: format!("predicate {} range type {} not defined", p.predicate_id, rt),
                    });
                }
            }
        }
        Ok(())
    }
}

fn compute_content_hash(
    types: &[enginer_domain::schema::TypeDefinition],
    predicates: &[enginer_domain::schema::PredicateDefinition],
) -> ContentHash {
    use blake2::digest::consts::U32;
    use blake2::{Blake2b, Digest};
    let mut h = Blake2b::<U32>::new();
    // Canonical JSON with sorted keys gives a stable hash.
    let mut ty_json: Vec<String> = types
        .iter()
        .map(|t| serde_json::to_string(t).unwrap_or_default())
        .collect();
    ty_json.sort();
    let mut pred_json: Vec<String> = predicates
        .iter()
        .map(|p| serde_json::to_string(p).unwrap_or_default())
        .collect();
    pred_json.sort();
    h.update(ty_json.join("\u{1f}").as_bytes());
    h.update(pred_json.join("\u{1f}").as_bytes());
    let out = h.finalize();
    let mut buf = [0u8; 32];
    buf.copy_from_slice(out.as_slice());
    ContentHash(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use enginer_domain::schema::{EntityClass, PredicateDefinition, PredicateDirection, TypeDefinition};
    use enginer_domain::{PredicateId, TypeId};

    #[test]
    fn publish_freezes_content_hash() {
        let tenant = TenantId::new();
        let mut d = SchemaDraft::new(tenant, None);
        let tid = TypeId::new();
        d.add_type(TypeDefinition {
            type_id: tid,
            stable_alias: "person".to_string(),
            entity_class: EntityClass::Data,
            predicates: Vec::new(),
            identity_policy_ids: Vec::new(),
        })
        .unwrap();
        d.freeze_for_validation().unwrap();
        let rev = d.publish().unwrap();
        assert_eq!(rev.state, SchemaRevisionState::Published);
        assert!(rev.content_hash.raw().iter().any(|b| *b != 0));
    }

    #[test]
    fn duplicate_type_rejected() {
        let tenant = TenantId::new();
        let mut d = SchemaDraft::new(tenant, None);
        let tid = TypeId::new();
        let ty = TypeDefinition {
            type_id: tid,
            stable_alias: "person".to_string(),
            entity_class: EntityClass::Data,
            predicates: Vec::new(),
            identity_policy_ids: Vec::new(),
        };
        d.add_type(ty.clone()).unwrap();
        assert!(d.add_type(ty).is_err());
    }

    #[test]
    fn predicate_range_must_exist() {
        let tenant = TenantId::new();
        let mut d = SchemaDraft::new(tenant, None);
        let pid = PredicateId::new();
        d.add_predicate(PredicateDefinition {
            predicate_id: pid,
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
}
