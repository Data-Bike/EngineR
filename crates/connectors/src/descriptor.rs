//! Connector descriptor and capabilities (docs/14 §1).
//!
//! A descriptor declares what the connector can do, its legal basis, and
//! supported operations. The core uses this to select connectors for checks.

use serde::{Deserialize, Serialize};

/// Kind of connector (docs/14 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    /// Internal EngineR database.
    Internal,
    /// External API (АСУ, Контур.Фокус, ФНС, etc.).
    Api,
    /// File import (Excel, CSV, NDJSON).
    FileImport,
    /// Internet / social media (high-risk class, docs/14 §13).
    Internet,
    /// Manual entry.
    Manual,
}

/// Capabilities declared by a connector (docs/14 §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorCapabilities {
    /// Whether the connector supports search operations.
    pub search: bool,
    /// Whether the connector supports fetch (single record) operations.
    pub fetch: bool,
    /// Whether the connector supports monitoring (periodic re-check).
    pub monitor: bool,
    /// Whether the connector supports file import.
    pub file_import: bool,
    /// Supported target type IDs (empty = any).
    pub supported_target_types: Vec<String>,
    /// Supported check kinds (sanction, pep, adverse_media, etc.).
    pub supported_check_kinds: Vec<String>,
    /// Maximum response size in bytes.
    pub max_response_bytes: u64,
    /// Whether raw payload is retained.
    pub retains_raw_payload: bool,
}

impl Default for ConnectorCapabilities {
    fn default() -> Self {
        Self {
            search: true,
            fetch: true,
            monitor: false,
            file_import: false,
            supported_target_types: Vec::new(),
            supported_check_kinds: Vec::new(),
            max_response_bytes: 10 * 1024 * 1024, // 10 MB
            retains_raw_payload: true,
        }
    }
}

/// Legal and licensing metadata (docs/14 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalMetadata {
    /// Legal basis for data collection (e.g. "consent", "legitimate_interest").
    pub legal_basis: String,
    /// Purpose limitation (e.g. "kyc", "aml", "fraud_prevention").
    pub purpose: String,
    /// Jurisdiction (e.g. "RU", "EU").
    pub jurisdiction: String,
    /// License reference (contract/agreement ID).
    pub license_ref: Option<String>,
    /// Data retention period in days (None = indefinite).
    pub retention_days: Option<u32>,
    /// Whether redistribution is allowed.
    pub redistribution_allowed: bool,
}

/// Connector descriptor (docs/14 §1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorDescriptor {
    /// Stable connector type ID.
    pub connector_type_id: String,
    /// Human-readable name.
    pub name: String,
    /// Connector version.
    pub version: String,
    /// Kind of connector.
    pub kind: ConnectorKind,
    /// Capabilities.
    pub capabilities: ConnectorCapabilities,
    /// Legal metadata.
    pub legal: LegalMetadata,
    /// Request schema version.
    pub request_schema_version: String,
    /// Response schema version.
    pub response_schema_version: String,
    /// Mapping package version.
    pub mapping_package_version: String,
    /// Trust rank (0=untrusted, 100=authoritative).
    pub trust_rank: u8,
}
