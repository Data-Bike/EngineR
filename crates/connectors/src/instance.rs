//! Connector instance configuration (docs/14 §3).
//!
//! Tenant-specific connector settings: environment, secrets, quotas, legal
//! basis. Secrets are available only to the runner and never returned by
//! Admin API (docs/14 §3).

use serde::{Deserialize, Serialize};

/// Trust rank of a connector instance (docs/14 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustRank {
    /// Untrusted (internet, social media).
    Untrusted = 0,
    /// Low trust (user-provided data).
    Low = 25,
    /// Medium trust (third-party APIs).
    Medium = 50,
    /// High trust (official registries).
    High = 75,
    /// Authoritative (government sources).
    Authoritative = 100,
}

impl TrustRank {
    /// Get the numeric rank value.
    pub fn rank(self) -> u8 {
        self as u8
    }
}

/// Retention policy for raw payloads (docs/14 §3, §14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// How long to retain raw payloads (days).
    pub raw_payload_retention_days: u32,
    /// How long to retain parsed observations (days).
    pub observation_retention_days: u32,
    /// Whether to apply legal hold (suspends expiry).
    pub legal_hold: bool,
    /// Whether to cryptographically delete on expiry.
    pub cryptographic_delete: bool,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            raw_payload_retention_days: 90,
            observation_retention_days: 365,
            legal_hold: false,
            cryptographic_delete: true,
        }
    }
}

/// Tenant-specific connector instance configuration (docs/14 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorInstanceConfig {
    /// Instance ID (UUID).
    pub instance_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Connector type ID (from descriptor).
    pub connector_type_id: String,
    /// Connector version.
    pub connector_version: String,
    /// Whether the instance is enabled.
    pub enabled: bool,
    /// Enabled operations (subset of capabilities).
    pub enabled_operations: Vec<String>,
    /// Environment: "production", "sandbox", "test".
    pub environment: String,
    /// Base URL (for API connectors).
    pub base_url: Option<String>,
    /// Proxy URL (if required).
    pub proxy_url: Option<String>,
    /// Secret references (vault keys, never the actual secrets).
    pub secret_references: Vec<String>,
    /// Organization account (if applicable).
    pub organization_account: Option<String>,
    /// Rate limit: requests per second.
    pub rate_limit_rps: u32,
    /// Concurrency limit: max parallel requests.
    pub concurrency_limit: u32,
    /// Timeout in milliseconds.
    pub timeout_ms: u64,
    /// Retry count.
    pub retry_count: u32,
    /// Retry backoff base in milliseconds.
    pub retry_backoff_ms: u64,
    /// Cache freshness in seconds (how long results are considered fresh).
    pub cache_freshness_seconds: u32,
    /// Legal basis for this instance.
    pub legal_basis: String,
    /// Permitted purpose.
    pub purpose: String,
    /// Permitted target type IDs.
    pub permitted_target_types: Vec<String>,
    /// Permitted target fields.
    pub permitted_target_fields: Vec<String>,
    /// Retention policy.
    pub retention: RetentionPolicy,
    /// Trust rank.
    pub trust_rank: TrustRank,
    /// Mapping package version.
    pub mapping_package_version: String,
}

/// A running connector instance (docs/14 §3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectorInstance {
    /// The configuration.
    pub config: ConnectorInstanceConfig,
    /// Whether the instance is currently healthy.
    pub healthy: bool,
    /// Last health check time.
    pub last_health_check: Option<chrono::DateTime<chrono::Utc>>,
    /// Current quota usage (requests in current window).
    pub quota_used: u32,
    /// Quota limit per window.
    pub quota_limit: u32,
}

impl ConnectorInstance {
    /// Create a new instance from config.
    pub fn new(config: ConnectorInstanceConfig) -> Self {
        let quota_limit = config.rate_limit_rps;
        Self {
            config,
            healthy: true,
            last_health_check: None,
            quota_used: 0,
            quota_limit,
        }
    }

    /// Check if quota is available.
    pub fn has_quota(&self) -> bool {
        self.quota_used < self.quota_limit
    }

    /// Consume one unit of quota.
    pub fn consume_quota(&mut self) -> bool {
        if self.has_quota() {
            self.quota_used += 1;
            true
        } else {
            false
        }
    }

    /// Reset quota (called at the start of each window).
    pub fn reset_quota(&mut self) {
        self.quota_used = 0;
    }

    /// Mark the instance as healthy/unhealthy.
    pub fn set_health(&mut self, healthy: bool) {
        self.healthy = healthy;
        self.last_health_check = Some(chrono::Utc::now());
    }
}
