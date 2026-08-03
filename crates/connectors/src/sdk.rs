//! Connector SDK trait (docs/14 §1).
//!
//! A connector implements this trait to integrate with the EngineR runner.
//! The runner calls `search`/`fetch`/`monitor` and handles rate limiting,
//! retry, and circuit breaking externally (docs/14 §4).

use crate::descriptor::ConnectorDescriptor;
use crate::error::ConnectorResult;
use crate::instance::ConnectorInstance;
use crate::observation::SourceObservation;
use serde::{Deserialize, Serialize};

/// Context passed to connector operations (docs/14 §4).
#[derive(Debug, Clone)]
pub struct ConnectorContext<'a> {
    /// The connector instance.
    pub instance: &'a ConnectorInstance,
    /// Tenant ID.
    pub tenant_id: String,
    /// Request trace ID (for observability).
    pub trace_id: String,
    /// Whether this is a dry run (no side effects).
    pub dry_run: bool,
}

/// Search query parameters (docs/14 §4 step 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Target type alias (e.g. "person", "legal_entity").
    pub target_type: String,
    /// Search parameters (predicate alias → value).
    pub params: Vec<SearchParam>,
    /// Maximum results to return.
    pub limit: u32,
    /// Whether to include raw payload in the response.
    pub include_raw: bool,
    /// Check kind (e.g. "sanction", "pep", "adverse_media").
    pub check_kind: Option<String>,
}

/// A single search parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchParam {
    /// Predicate alias (e.g. "person.inn", "legal_entity.full_name").
    pub predicate_alias: String,
    /// Value to search for.
    pub value: String,
    /// Match mode: "exact", "prefix", "fuzzy".
    pub match_mode: String,
}

/// Search response from a connector (docs/14 §4 step 7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResponse {
    /// Observations matching the query.
    pub observations: Vec<SourceObservation>,
    /// Total matches available (may be more than returned).
    pub total_available: u32,
    /// Whether the response is from cache.
    pub from_cache: bool,
    /// Cache freshness timestamp.
    pub cache_fresh_until: Option<chrono::DateTime<chrono::Utc>>,
    /// Source-side quota remaining (if reported).
    pub source_quota_remaining: Option<u32>,
}

/// The connector SDK trait (docs/14 §1).
///
/// Implementors provide the descriptor, credential validation, health check,
/// and search/fetch/monitor operations. The runner handles rate limiting,
/// retry, and circuit breaking.
pub trait Connector: Send + Sync {
    /// Get the connector descriptor.
    fn descriptor(&self) -> &ConnectorDescriptor;

    /// Validate credentials (docs/14 §4 step 2).
    fn validate_credentials(&self, ctx: &ConnectorContext<'_>) -> ConnectorResult<()>;

    /// Check health and quota (docs/14 §4 step 3).
    fn health_check(&self, ctx: &ConnectorContext<'_>) -> ConnectorResult<HealthStatus>;

    /// Search for entities (docs/14 §4 step 1).
    fn search(
        &self,
        ctx: &ConnectorContext<'_>,
        query: &SearchQuery,
    ) -> ConnectorResult<SearchResponse>;

    /// Fetch a single record by source ID (docs/14 §4 step 5).
    fn fetch(
        &self,
        ctx: &ConnectorContext<'_>,
        source_record_id: &str,
    ) -> ConnectorResult<SourceObservation>;

    /// Monitor: check for changes since last observation (docs/14 §4 step 5).
    fn monitor(
        &self,
        ctx: &ConnectorContext<'_>,
        last_observation_id: &str,
    ) -> ConnectorResult<SearchResponse>;
}

/// Health status returned by `health_check` (docs/14 §4 step 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Whether the connector is healthy.
    pub healthy: bool,
    /// Quota remaining (requests in current window).
    pub quota_remaining: u32,
    /// Quota reset time (Unix timestamp ms).
    pub quota_reset_at_ms: u64,
    /// Optional message (e.g. "degraded", "maintenance").
    pub message: Option<String>,
}
