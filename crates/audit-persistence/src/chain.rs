//! Tamper-evident chain hashing (docs/08 §11).
//!
//! Each entry's hash includes the previous entry's hash, forming a chain.
//! Uses BLAKE2b-256 for cryptographic hashing. Checkpoints periodically
//! persist the latest hash for verification.

use crate::error::{AuditPersistenceError, AuditPersistenceResult};
use blake2::Blake2bVar;
use blake2::digest::{Update, VariableOutput};

/// Chain state: tracks the last sequence and hash for the next entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ChainState {
    /// Last sequence number assigned.
    pub last_sequence: i64,
    /// Last entry hash (hex-encoded).
    pub last_hash: String,
}

impl Default for ChainState {
    fn default() -> Self {
        Self {
            last_sequence: 0,
            last_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
        }
    }
}

impl ChainState {
    /// Create a new empty chain state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a chain state from a checkpoint.
    pub fn from_checkpoint(sequence: i64, hash: &str) -> Self {
        Self {
            last_sequence: sequence,
            last_hash: hash.to_string(),
        }
    }

    /// Advance to the next sequence.
    pub fn next_sequence(&self) -> i64 {
        self.last_sequence + 1
    }
}

/// Chain hasher: computes BLAKE2b-256 hashes for audit entries.
pub struct ChainHasher;

impl ChainHasher {
    /// Compute the hash for an audit entry.
    ///
    /// The hash includes: sequence, previous_hash, timestamp, tenant_id,
    /// actor, action, kind, resource_kind, resource_id, decision/effect.
    /// This forms a tamper-evident chain where modifying any field
    /// invalidates the hash.
    pub fn compute_hash(
        sequence: i64,
        previous_hash: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
        tenant_id: &str,
        actor: &str,
        action: &str,
        kind: &str,
        resource_kind: &str,
        resource_id: Option<&str>,
        effect: Option<&str>,
    ) -> AuditPersistenceResult<String> {
        let mut hasher = Blake2bVar::new(32)
            .map_err(|e| AuditPersistenceError::Hash { message: e.to_string() })?;
        // Sequence (big-endian for consistent ordering)
        hasher.update(&sequence.to_be_bytes());
        // Previous hash (links the chain)
        hasher.update(previous_hash.as_bytes());
        // Timestamp (RFC3339 for consistent encoding)
        hasher.update(timestamp.to_rfc3339().as_bytes());
        // Tenant ID
        hasher.update(tenant_id.as_bytes());
        // Actor
        hasher.update(actor.as_bytes());
        // Action
        hasher.update(action.as_bytes());
        // Kind
        hasher.update(kind.as_bytes());
        // Resource kind
        hasher.update(resource_kind.as_bytes());
        // Resource ID (optional)
        if let Some(rid) = resource_id {
            hasher.update(rid.as_bytes());
        }
        // Effect/decision (optional)
        if let Some(eff) = effect {
            hasher.update(eff.as_bytes());
        }

        let mut result = vec![0u8; 32];
        hasher.finalize_variable(&mut result)
            .map_err(|e| AuditPersistenceError::Hash { message: e.to_string() })?;
        Ok(hex::encode(&result))
    }

    /// Verify that an entry's hash matches the computed hash.
    pub fn verify_hash(
        entry_hash: &str,
        sequence: i64,
        previous_hash: &str,
        timestamp: chrono::DateTime<chrono::Utc>,
        tenant_id: &str,
        actor: &str,
        action: &str,
        kind: &str,
        resource_kind: &str,
        resource_id: Option<&str>,
        effect: Option<&str>,
    ) -> AuditPersistenceResult<bool> {
        let computed = Self::compute_hash(
            sequence,
            previous_hash,
            timestamp,
            tenant_id,
            actor,
            action,
            kind,
            resource_kind,
            resource_id,
            effect,
        )?;
        Ok(computed == entry_hash)
    }

    /// Verify a chain of entries (each entry's previous_hash must match
    /// the prior entry's hash, and each hash must be valid).
    pub fn verify_chain(entries: &[ChainEntry]) -> AuditPersistenceResult<bool> {
        let mut expected_hash =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();
        let mut expected_seq: i64 = 0;

        for entry in entries {
            if entry.sequence != expected_seq {
                return Ok(false);
            }
            if entry.previous_hash != expected_hash {
                return Ok(false);
            }
            let computed = Self::compute_hash(
                entry.sequence,
                &entry.previous_hash,
                entry.timestamp,
                &entry.tenant_id,
                &entry.actor,
                &entry.action,
                &entry.kind,
                &entry.resource_kind,
                entry.resource_id.as_deref(),
                entry.effect.as_deref(),
            )?;
            if computed != entry.entry_hash {
                return Ok(false);
            }
            expected_hash = entry.entry_hash.clone();
            expected_seq += 1;
        }

        Ok(true)
    }
}

/// A chain entry for verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChainEntry {
    pub sequence: i64,
    pub previous_hash: String,
    pub entry_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tenant_id: String,
    pub actor: String,
    pub action: String,
    pub kind: String,
    pub resource_kind: String,
    pub resource_id: Option<String>,
    pub effect: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_state_default() {
        let state = ChainState::default();
        assert_eq!(state.last_sequence, 0);
        assert_eq!(
            state.last_hash,
            "0000000000000000000000000000000000000000000000000000000000000000"
        );
    }

    #[test]
    fn chain_state_next_sequence() {
        let state = ChainState::from_checkpoint(5, "abc123");
        assert_eq!(state.next_sequence(), 6);
    }

    #[test]
    fn chain_hasher_compute_hash_deterministic() {
        let ts = chrono::Utc::now();
        let hash1 = ChainHasher::compute_hash(
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        let hash2 = ChainHasher::compute_hash(
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn chain_hasher_different_inputs_different_hashes() {
        let ts = chrono::Utc::now();
        let hash1 = ChainHasher::compute_hash(
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        let hash2 = ChainHasher::compute_hash(
            2, // different sequence
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn chain_hasher_verify_hash_valid() {
        let ts = chrono::Utc::now();
        let hash = ChainHasher::compute_hash(
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        let valid = ChainHasher::verify_hash(
            &hash,
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        assert!(valid);
    }

    #[test]
    fn chain_hasher_verify_hash_invalid() {
        let ts = chrono::Utc::now();
        let valid = ChainHasher::verify_hash(
            "invalid_hash",
            1,
            "0000000000000000000000000000000000000000000000000000000000000000",
            ts,
            "tenant-A",
            "user-001",
            "entity.read",
            "decision",
            "entity",
            Some("ent-001"),
            Some("allow"),
        )
        .unwrap();
        assert!(!valid);
    }

    #[test]
    fn chain_hasher_verify_chain_valid() {
        let ts1 = chrono::Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(1);
        let genesis =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let hash1 = ChainHasher::compute_hash(
            0, genesis, ts1, "t", "u", "a", "decision", "entity", None, Some("allow"),
        )
        .unwrap();
        let hash2 = ChainHasher::compute_hash(
            1, &hash1, ts2, "t", "u", "a", "decision", "entity", None, Some("deny"),
        )
        .unwrap();

        let entries = vec![
            ChainEntry {
                sequence: 0,
                previous_hash: genesis.to_string(),
                entry_hash: hash1.clone(),
                timestamp: ts1,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "a".to_string(),
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("allow".to_string()),
            },
            ChainEntry {
                sequence: 1,
                previous_hash: hash1,
                entry_hash: hash2.clone(),
                timestamp: ts2,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "a".to_string(),
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("deny".to_string()),
            },
        ];

        assert!(ChainHasher::verify_chain(&entries).unwrap());
    }

    #[test]
    fn chain_hasher_verify_chain_tampered() {
        let ts1 = chrono::Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(1);
        let genesis =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let hash1 = ChainHasher::compute_hash(
            0, genesis, ts1, "t", "u", "a", "decision", "entity", None, Some("allow"),
        )
        .unwrap();
        let hash2 = ChainHasher::compute_hash(
            1, &hash1, ts2, "t", "u", "a", "decision", "entity", None, Some("deny"),
        )
        .unwrap();

        // Tamper: change the action in the first entry
        let entries = vec![
            ChainEntry {
                sequence: 0,
                previous_hash: genesis.to_string(),
                entry_hash: hash1.clone(),
                timestamp: ts1,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "tampered".to_string(), // TAMPERED
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("allow".to_string()),
            },
            ChainEntry {
                sequence: 1,
                previous_hash: hash1,
                entry_hash: hash2,
                timestamp: ts2,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "a".to_string(),
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("deny".to_string()),
            },
        ];

        // Chain should be broken because the first entry's hash doesn't
        // match the computed hash with "tampered" action.
        assert!(!ChainHasher::verify_chain(&entries).unwrap());
    }

    #[test]
    fn chain_hasher_verify_chain_broken_link() {
        let ts1 = chrono::Utc::now();
        let ts2 = ts1 + chrono::Duration::seconds(1);
        let genesis =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let hash1 = ChainHasher::compute_hash(
            0, genesis, ts1, "t", "u", "a", "decision", "entity", None, Some("allow"),
        )
        .unwrap();
        // Use a different previous_hash for entry 2 (broken link)
        let hash2 = ChainHasher::compute_hash(
            1, "wrong_hash", ts2, "t", "u", "a", "decision", "entity", None, Some("deny"),
        )
        .unwrap();

        let entries = vec![
            ChainEntry {
                sequence: 0,
                previous_hash: genesis.to_string(),
                entry_hash: hash1,
                timestamp: ts1,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "a".to_string(),
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("allow".to_string()),
            },
            ChainEntry {
                sequence: 1,
                previous_hash: "wrong_hash".to_string(), // BROKEN LINK
                entry_hash: hash2,
                timestamp: ts2,
                tenant_id: "t".to_string(),
                actor: "u".to_string(),
                action: "a".to_string(),
                kind: "decision".to_string(),
                resource_kind: "entity".to_string(),
                resource_id: None,
                effect: Some("deny".to_string()),
            },
        ];

        assert!(!ChainHasher::verify_chain(&entries).unwrap());
    }

    #[test]
    fn chain_hasher_verify_chain_sequence_gap() {
        let ts1 = chrono::Utc::now();
        let genesis =
            "0000000000000000000000000000000000000000000000000000000000000000";
        let hash1 = ChainHasher::compute_hash(
            0, genesis, ts1, "t", "u", "a", "decision", "entity", None, Some("allow"),
        )
        .unwrap();

        let entries = vec![ChainEntry {
            sequence: 5, // GAP: expected 0
            previous_hash: genesis.to_string(),
            entry_hash: hash1,
            timestamp: ts1,
            tenant_id: "t".to_string(),
            actor: "u".to_string(),
            action: "a".to_string(),
            kind: "decision".to_string(),
            resource_kind: "entity".to_string(),
            resource_id: None,
            effect: Some("allow".to_string()),
        }];

        assert!(!ChainHasher::verify_chain(&entries).unwrap());
    }

    #[test]
    fn chain_hasher_verify_empty_chain() {
        // Empty chain is valid (no entries to verify).
        assert!(ChainHasher::verify_chain(&[]).unwrap());
    }
}
