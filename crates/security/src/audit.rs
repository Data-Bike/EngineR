//! Audit log (docs/08 §11).
//!
//! Records: authentication, decision, denied request, policy change/
//! simulation, export, blob read, bulk action, break-glass (docs/08 §11).
//!
//! Fields: timestamp, tenant, actor/effective actor, action/resource/
//! query fingerprint, decision/effect/reason IDs, policy revision/IDs,
//! request/correlation, row count/bytes, client/service, tamper-evident
//! chain/checkpoint (docs/08 §11).
//!
//! Audit does not store password/token and minimizes PII (docs/08 §11).

use serde::{Deserialize, Serialize};

/// Audit event kind (docs/08 §11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditKind {
    /// Authentication event (login, logout, token refresh).
    Authentication,
    /// Policy decision (allow/deny).
    Decision,
    /// Denied request.
    DeniedRequest,
    /// Policy change or simulation (docs/08 §14).
    PolicyChange,
    /// Export operation.
    Export,
    /// Blob/file read.
    BlobRead,
    /// Bulk action (import/mutate).
    BulkAction,
    /// Break-glass activation (docs/08 §13).
    BreakGlass,
    /// Impersonation/delegation (docs/08 §12).
    Impersonation,
}

/// An audit log entry (docs/08 §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Entry ID (UUID).
    pub entry_id: String,
    /// Tamper-evident chain sequence number.
    pub sequence: u64,
    /// Previous entry hash (for tamper-evident chain).
    pub previous_hash: String,
    /// This entry's hash.
    pub hash: String,
    /// Timestamp.
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Tenant ID.
    pub tenant_id: String,
    /// Actor (user/service who performed the action).
    pub actor: String,
    /// Effective actor (if impersonating, docs/08 §12).
    pub effective_actor: Option<String>,
    /// Audit kind.
    pub kind: AuditKind,
    /// Action performed.
    pub action: String,
    /// Resource type (if applicable).
    pub resource_type: Option<String>,
    /// Resource ID (if applicable).
    pub resource_id: Option<String>,
    /// Query fingerprint (for query operations).
    pub query_fingerprint: Option<String>,
    /// Decision/effect (allow/deny).
    pub decision: Option<String>,
    /// Reason IDs (policy rule IDs that contributed).
    pub reason_ids: Vec<String>,
    /// Policy revision ID.
    pub policy_revision_id: Option<String>,
    /// Request ID (correlation).
    pub request_id: Option<String>,
    /// Correlation ID.
    pub correlation_id: Option<String>,
    /// Row count affected (for bulk/query operations).
    pub row_count: Option<u64>,
    /// Bytes transferred (for export/blob operations).
    pub bytes: Option<u64>,
    /// Client (user agent / service name).
    pub client: Option<String>,
    /// Service (if called by another service).
    pub service: Option<String>,
    /// Additional metadata (minimized, no PII/passwords, docs/08 §11).
    pub metadata: Vec<(String, String)>,
}

impl AuditEntry {
    /// Create a new audit entry (without hash — caller must compute it).
    pub fn new(
        sequence: u64,
        previous_hash: &str,
        tenant_id: &str,
        actor: &str,
        kind: AuditKind,
        action: &str,
    ) -> Self {
        Self {
            entry_id: uuid::Uuid::now_v7().to_string(),
            sequence,
            previous_hash: previous_hash.to_string(),
            hash: String::new(), // computed by TamperChain
            timestamp: chrono::Utc::now(),
            tenant_id: tenant_id.to_string(),
            actor: actor.to_string(),
            effective_actor: None,
            kind,
            action: action.to_string(),
            resource_type: None,
            resource_id: None,
            query_fingerprint: None,
            decision: None,
            reason_ids: Vec::new(),
            policy_revision_id: None,
            request_id: None,
            correlation_id: None,
            row_count: None,
            bytes: None,
            client: None,
            service: None,
            metadata: Vec::new(),
        }
    }

    /// Set the effective actor (for impersonation, docs/08 §12).
    pub fn with_effective_actor(mut self, actor: &str) -> Self {
        self.effective_actor = Some(actor.to_string());
        self
    }

    /// Set the resource.
    pub fn with_resource(mut self, resource_type: &str, resource_id: &str) -> Self {
        self.resource_type = Some(resource_type.to_string());
        self.resource_id = Some(resource_id.to_string());
        self
    }

    /// Set the decision.
    pub fn with_decision(mut self, decision: &str, reason_ids: Vec<String>) -> Self {
        self.decision = Some(decision.to_string());
        self.reason_ids = reason_ids;
        self
    }

    /// Set the policy revision.
    pub fn with_policy_revision(mut self, revision_id: &str) -> Self {
        self.policy_revision_id = Some(revision_id.to_string());
        self
    }

    /// Set the request/correlation IDs.
    pub fn with_request(mut self, request_id: &str, correlation_id: Option<&str>) -> Self {
        self.request_id = Some(request_id.to_string());
        self.correlation_id = correlation_id.map(|s| s.to_string());
        self
    }

    /// Set the row count and bytes.
    pub fn with_metrics(mut self, row_count: Option<u64>, bytes: Option<u64>) -> Self {
        self.row_count = row_count;
        self.bytes = bytes;
        self
    }

    /// Set the client/service.
    pub fn with_client(mut self, client: &str, service: Option<&str>) -> Self {
        self.client = Some(client.to_string());
        self.service = service.map(|s| s.to_string());
        self
    }

    /// Add metadata.
    pub fn add_metadata(&mut self, key: &str, value: &str) {
        self.metadata.push((key.to_string(), value.to_string()));
    }

    /// Compute the hash for this entry (simple hash for tamper-evidence).
    pub fn compute_hash(&self) -> String {
        // Simple hash: combine sequence + previous_hash + timestamp + action
        // In production, this would use a cryptographic hash (SHA-256/blake2b)
        let input = format!(
            "{}:{}:{}:{}:{}:{}",
            self.sequence,
            self.previous_hash,
            self.timestamp,
            self.tenant_id,
            self.actor,
            self.action
        );
        // Simple FNV-1a-like hash for testing
        let mut hash: u64 = 0xcbf29ce484222325;
        for b in input.bytes() {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:016x}", hash)
    }
}

/// Tamper-evident chain (docs/08 §11).
///
/// Each entry's hash includes the previous entry's hash, forming a chain.
/// Checkpoints periodically persist the latest hash for verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TamperChain {
    /// Current sequence number.
    pub sequence: u64,
    /// Last entry hash.
    pub last_hash: String,
    /// Checkpoint hashes (sequence → hash).
    pub checkpoints: Vec<(u64, String)>,
    /// Total entries.
    pub total_entries: u64,
}

impl Default for TamperChain {
    fn default() -> Self {
        Self {
            sequence: 0,
            last_hash: "0000000000000000".to_string(),
            checkpoints: Vec::new(),
            total_entries: 0,
        }
    }
}

impl TamperChain {
    /// Create a new empty tamper-evident chain.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append an entry to the chain (computes hash, updates state).
    pub fn append(&mut self, mut entry: AuditEntry) -> AuditEntry {
        entry.sequence = self.sequence;
        entry.previous_hash = self.last_hash.clone();
        entry.hash = entry.compute_hash();
        self.last_hash = entry.hash.clone();
        self.sequence += 1;
        self.total_entries += 1;
        entry
    }

    /// Create a checkpoint at the current sequence.
    pub fn checkpoint(&mut self) {
        self.checkpoints
            .push((self.sequence, self.last_hash.clone()));
    }

    /// Verify the chain integrity (returns true if all entries are consistent).
    pub fn verify(&self, entries: &[AuditEntry]) -> bool {
        let mut expected_hash = "0000000000000000".to_string();
        let mut expected_seq: u64 = 0;
        for entry in entries {
            if entry.sequence != expected_seq {
                return false;
            }
            if entry.previous_hash != expected_hash {
                return false;
            }
            let computed = entry.compute_hash();
            if entry.hash != computed {
                return false;
            }
            expected_hash = entry.hash.clone();
            expected_seq += 1;
        }
        expected_hash == self.last_hash
    }

    /// Get the latest checkpoint.
    pub fn latest_checkpoint(&self) -> Option<&(u64, String)> {
        self.checkpoints.last()
    }
}

/// Audit query (docs/08 §11).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Tenant ID (required for isolation).
    pub tenant_id: String,
    /// Filter by kind (optional).
    pub kind: Option<AuditKind>,
    /// Filter by actor (optional).
    pub actor: Option<String>,
    /// Filter by action (optional).
    pub action: Option<String>,
    /// Filter by resource type (optional).
    pub resource_type: Option<String>,
    /// Filter by resource ID (optional).
    pub resource_id: Option<String>,
    /// Filter by decision (optional).
    pub decision: Option<String>,
    /// Filter from timestamp.
    pub from: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter to timestamp.
    pub to: Option<chrono::DateTime<chrono::Utc>>,
    /// Limit (max entries to return).
    pub limit: Option<u32>,
    /// Cursor for pagination (sequence number).
    pub cursor: Option<u64>,
}

/// Audit retention policy (docs/08 §11, §15).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRetention {
    /// Retention period in days.
    pub retention_days: u32,
    /// Whether legal hold is applied (suspends retention).
    pub legal_hold: bool,
    /// Whether to archive old entries before deletion.
    pub archive_before_delete: bool,
    /// Archive storage URL (if archiving).
    pub archive_url: Option<String>,
}

impl AuditRetention {
    /// Create a default retention policy (1 year, no legal hold).
    pub fn default_policy() -> Self {
        Self {
            retention_days: 365,
            legal_hold: false,
            archive_before_delete: true,
            archive_url: None,
        }
    }

    /// Check if an entry is expired given the retention policy.
    pub fn is_expired(&self, entry: &AuditEntry) -> bool {
        if self.legal_hold {
            return false;
        }
        let expiry = entry.timestamp + chrono::Duration::days(self.retention_days as i64);
        chrono::Utc::now() > expiry
    }
}
