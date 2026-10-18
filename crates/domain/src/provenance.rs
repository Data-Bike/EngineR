//! Provenance (docs/03 §8).
//!
//! External facts carry connector/source record, fetch/effective timestamps,
//! raw payload hash, legal basis, parser/mapping version, actor/job/correlation,
//! confidence/verification and evidence blobs. Sources never overwrite each
//! other; a resolution policy forms the effective view and records conflicts.

use crate::ids::{ContentHash, CorrelationId, JobId, PrincipalId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reference to a source record / connector observation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRef {
    /// Connector / source system instance id.
    pub source_system_id: uuid::Uuid,
    /// Namespace within the source (e.g. table name, dataset).
    pub namespace: String,
    /// External id within the source.
    pub external_id: String,
    /// Observation scope (e.g. `per_observation`, `per_source`).
    pub observation_scope: String,
}

/// A provenance reference attached to a statement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceRef {
    /// Source record reference, if external.
    pub source: Option<SourceRef>,
    /// When the fact was fetched from the source.
    pub fetched_at: Option<DateTime<Utc>>,
    /// When the fact is considered effective in the domain.
    pub effective_at: Option<DateTime<Utc>>,
    /// Hash of the raw payload (BLAKE2b-256).
    pub raw_payload_hash: Option<ContentHash>,
    /// Legal basis / license reference.
    pub legal_basis: Option<String>,
    /// Parser / mapping version that produced the canonical value.
    pub parser_version: Option<String>,
    /// Actor that recorded the statement.
    pub actor: Option<PrincipalId>,
    /// Job that produced the statement (imports/monitors).
    pub job: Option<JobId>,
    /// Correlation id of the flow.
    pub correlation: Option<CorrelationId>,
    /// Confidence bucket (`high`/`medium`/`low` or domain-specific).
    pub confidence: Option<String>,
    /// Verification status (`verified`/`unverified`/`disputed`).
    pub verification: Option<String>,
}

impl ProvenanceRef {
    /// Provenance for a user-initiated statement.
    pub fn user(actor: Option<uuid::Uuid>) -> Self {
        Self {
            source: None,
            fetched_at: None,
            effective_at: None,
            raw_payload_hash: None,
            legal_basis: None,
            parser_version: None,
            actor: actor.map(PrincipalId),
            job: None,
            correlation: None,
            confidence: Some("high".to_string()),
            verification: Some("unverified".to_string()),
        }
    }

    /// Provenance for a connector/source observation.
    #[must_use]
    pub fn source(source: SourceRef, fetched_at: DateTime<Utc>) -> Self {
        Self {
            source: Some(source),
            fetched_at: Some(fetched_at),
            effective_at: Some(fetched_at),
            raw_payload_hash: None,
            legal_basis: None,
            parser_version: None,
            actor: None,
            job: None,
            correlation: None,
            confidence: Some("medium".to_string()),
            verification: Some("unverified".to_string()),
        }
    }
}
