//! Source observation and mapping result (docs/14 §1, §6).
//!
//! A connector returns `SourceObservation` — typed data extracted from a
//! source response. The mapper links predicates with provenance. Raw payload
//! is stored encrypted per policy (docs/14 §4 step 6).

use serde::{Deserialize, Serialize};

/// Status of an observation (docs/14 §4 step 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationStatus {
    /// Observation is fresh and valid.
    Fresh,
    /// Observation is from cache and may be stale.
    Cached,
    /// Observation is partial (source returned incomplete data).
    Partial,
    /// Observation indicates no match (negative result).
    NoMatch,
    /// Observation is ambiguous (multiple candidates).
    Ambiguous,
}

/// Policy for raw payload storage (docs/14 §4 step 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawPayloadPolicy {
    /// Store raw payload encrypted.
    Store,
    /// Store only hash, discard payload.
    HashOnly,
    /// Discard entirely (no retention).
    Discard,
}

/// Provenance metadata for an observation (docs/14 §4 step 8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceMeta {
    /// Source connector type ID.
    pub source_connector_type_id: String,
    /// Source record ID (external ID in the source system).
    pub source_record_id: String,
    /// Time when the source returned the data.
    pub retrieved_at: chrono::DateTime<chrono::Utc>,
    /// Source URL (if applicable, e.g. for internet sources).
    pub source_url: Option<String>,
    /// Content hash of the raw payload (blake2b).
    pub content_hash: String,
    /// Trust rank of the source (0-100).
    pub trust_rank: u8,
    /// Purpose of this data collection.
    pub purpose: String,
}

/// A typed observation from a source (docs/14 §1, §4 step 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObservation {
    /// Observation ID (UUID).
    pub observation_id: String,
    /// Status of the observation.
    pub status: ObservationStatus,
    /// Provenance metadata.
    pub provenance: ProvenanceMeta,
    /// Typed data as JSON (predicate → value).
    pub data: serde_json::Value,
    /// Whether the observation is adverse (negative finding).
    pub adverse: bool,
    /// Confidence score (0.0 - 1.0, stored as string for decimal precision).
    pub confidence: String,
    /// Raw payload policy applied.
    pub raw_payload_policy: RawPayloadPolicy,
    /// Raw payload hash (if stored).
    pub raw_payload_hash: Option<String>,
    /// Redacted diagnostics (no secrets, docs/14 §1).
    pub diagnostics: Option<String>,
}

/// Result of mapping observations to predicates (docs/14 §4 step 8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappingResult {
    /// Mapped predicate values.
    pub mapped_predicates: Vec<MappedPredicate>,
    /// Unmapped fields (source field → reason).
    pub unmapped: Vec<UnmappedField>,
    /// Identity resolution result.
    pub identity_resolution: IdentityResolution,
}

/// A mapped predicate value with provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MappedPredicate {
    /// Target predicate ID.
    pub predicate_id: String,
    /// Target predicate alias (ergonomic).
    pub predicate_alias: String,
    /// Canonical value (string representation).
    pub canonical_value: String,
    /// Value kind (text, date, money, etc.).
    pub value_kind: String,
    /// Provenance reference (observation ID).
    pub observation_id: String,
    /// Source field name in the original response.
    pub source_field: String,
}

/// An unmapped field from the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnmappedField {
    /// Source field name.
    pub source_field: String,
    /// Reason it was not mapped.
    pub reason: String,
}

/// Identity resolution result (docs/14 §4 step 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityResolution {
    /// "exact_link", "candidate", "no_match"
    pub outcome: String,
    /// Resolved entity public ID (if exact link).
    pub entity_public_id: Option<String>,
    /// Candidate entity public IDs (if fuzzy).
    pub candidates: Vec<String>,
    /// Match score (0.0 - 1.0, string for decimal precision).
    pub score: Option<String>,
}
