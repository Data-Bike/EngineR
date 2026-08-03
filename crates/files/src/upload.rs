//! Staged upload protocol (docs/15 §1, §3 step 1).
//!
//! Upload is a job, separated from processing. Staged upload:
//! 1. init: create upload session with expected size/hash/metadata;
//! 2. chunk: upload chunks (resumable, idempotent);
//! 3. complete: verify checksum, trigger scan;
//! 4. abort: discard upload.

use crate::error::{FileError, FileResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Upload status (docs/15 §3 step 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    /// Upload initialized, awaiting chunks.
    Initiated,
    /// Chunks being uploaded.
    Uploading,
    /// All chunks uploaded, awaiting completion.
    AllChunksReceived,
    /// Upload completed, file scanned and stored.
    Completed,
    /// Upload aborted.
    Aborted,
    /// Scan failed or rejected.
    ScanFailed,
}

/// Upload initialization request (docs/15 §3 step 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadInit {
    /// Tenant ID.
    pub tenant_id: String,
    /// Original filename.
    pub filename: String,
    /// Expected content type (MIME).
    pub content_type: String,
    /// Expected total size in bytes.
    pub expected_size: u64,
    /// Expected content hash (blake2b or sha256).
    pub expected_hash: String,
    /// Hash algorithm used.
    pub hash_algorithm: String,
    /// Chunk size in bytes (for resumable upload).
    pub chunk_size: u64,
    /// User metadata (key → value).
    pub metadata: HashMap<String, String>,
    /// Storage class (hot/cold/archive).
    pub storage_class: String,
}

/// A single uploaded chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadChunk {
    /// Chunk index (0-based).
    pub index: u32,
    /// Chunk size in bytes.
    pub size: u64,
    /// Chunk content hash (for verification).
    pub hash: String,
    /// Whether this is the last chunk.
    pub is_last: bool,
}

/// Upload completion request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadComplete {
    /// Final content hash (computed from all chunks).
    pub final_hash: String,
    /// Total size uploaded.
    pub total_size: u64,
}

/// Upload abort request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UploadAbort {
    /// Reason for abort.
    pub reason: String,
}

/// A staged upload session (docs/15 §3 step 1).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StagedUpload {
    /// Upload ID (UUID).
    pub upload_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Original filename.
    pub filename: String,
    /// Content type (MIME).
    pub content_type: String,
    /// Expected total size.
    pub expected_size: u64,
    /// Expected hash.
    pub expected_hash: String,
    /// Hash algorithm.
    pub hash_algorithm: String,
    /// Chunk size.
    pub chunk_size: u64,
    /// Total expected chunks (computed from size + chunk_size).
    pub total_chunks: u32,
    /// Received chunks (index → chunk metadata).
    pub received_chunks: HashMap<u32, UploadChunk>,
    /// User metadata.
    pub metadata: HashMap<String, String>,
    /// Storage class.
    pub storage_class: String,
    /// Current status.
    pub status: UploadStatus,
    /// Created at.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated at.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Final object ID (after completion).
    pub object_id: Option<String>,
    /// Final content hash (after completion).
    pub final_hash: Option<String>,
    /// Total size uploaded (after completion).
    pub total_size: Option<u64>,
}

impl StagedUpload {
    /// Initialize a new staged upload.
    pub fn init(req: UploadInit) -> FileResult<Self> {
        if req.expected_size == 0 {
            return Err(FileError::InvalidConfig {
                message: "expected_size must be > 0".to_string(),
            });
        }
        if req.chunk_size == 0 {
            return Err(FileError::InvalidConfig {
                message: "chunk_size must be > 0".to_string(),
            });
        }
        let total_chunks = ((req.expected_size + req.chunk_size - 1) / req.chunk_size) as u32;
        let now = chrono::Utc::now();
        Ok(Self {
            upload_id: uuid::Uuid::now_v7().to_string(),
            tenant_id: req.tenant_id,
            filename: req.filename,
            content_type: req.content_type,
            expected_size: req.expected_size,
            expected_hash: req.expected_hash,
            hash_algorithm: req.hash_algorithm,
            chunk_size: req.chunk_size,
            total_chunks,
            received_chunks: HashMap::new(),
            metadata: req.metadata,
            storage_class: req.storage_class,
            status: UploadStatus::Initiated,
            created_at: now,
            updated_at: now,
            object_id: None,
            final_hash: None,
            total_size: None,
        })
    }

    /// Receive a chunk (idempotent — re-uploading same chunk is OK).
    pub fn receive_chunk(&mut self, chunk: UploadChunk) -> FileResult<()> {
        if self.status == UploadStatus::Aborted || self.status == UploadStatus::Completed {
            return Err(FileError::Upload {
                message: format!("cannot receive chunk: upload is {:?}", self.status),
            });
        }
        if chunk.index >= self.total_chunks {
            return Err(FileError::Upload {
                message: format!(
                    "chunk index {} out of range (total {})",
                    chunk.index, self.total_chunks
                ),
            });
        }
        // Idempotent: if already received with same hash, skip
        if let Some(existing) = self.received_chunks.get(&chunk.index) {
            if existing.hash == chunk.hash {
                return Ok(());
            }
            return Err(FileError::Upload {
                message: format!(
                    "chunk {} hash mismatch: expected {}, got {}",
                    chunk.index, existing.hash, chunk.hash
                ),
            });
        }
        self.received_chunks.insert(chunk.index, chunk);
        self.status = if self.received_chunks.len() == self.total_chunks as usize {
            UploadStatus::AllChunksReceived
        } else {
            UploadStatus::Uploading
        };
        self.touch();
        Ok(())
    }

    /// Complete the upload (verify checksum).
    pub fn complete(&mut self, req: UploadComplete) -> FileResult<()> {
        if self.status != UploadStatus::AllChunksReceived {
            return Err(FileError::Upload {
                message: format!("cannot complete: status is {:?}", self.status),
            });
        }
        if req.final_hash != self.expected_hash {
            return Err(FileError::ChecksumMismatch {
                expected: self.expected_hash.clone(),
                got: req.final_hash,
            });
        }
        if req.total_size != self.expected_size {
            return Err(FileError::Upload {
                message: format!(
                    "size mismatch: expected {}, got {}",
                    self.expected_size, req.total_size
                ),
            });
        }
        self.status = UploadStatus::Completed;
        self.final_hash = Some(req.final_hash);
        self.total_size = Some(req.total_size);
        self.object_id = Some(uuid::Uuid::now_v7().to_string());
        self.touch();
        Ok(())
    }

    /// Abort the upload.
    pub fn abort(&mut self, req: UploadAbort) -> FileResult<()> {
        if self.status == UploadStatus::Completed {
            return Err(FileError::Upload {
                message: "cannot abort completed upload".to_string(),
            });
        }
        self.status = UploadStatus::Aborted;
        self.received_chunks.clear();
        let _ = req.reason; // logged but not stored
        self.touch();
        Ok(())
    }

    /// Get upload progress (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.total_chunks == 0 {
            return 0.0;
        }
        self.received_chunks.len() as f64 / self.total_chunks as f64
    }

    /// Check if all chunks are received.
    pub fn is_complete(&self) -> bool {
        self.received_chunks.len() == self.total_chunks as usize
    }

    /// Check if the upload is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            UploadStatus::Completed | UploadStatus::Aborted | UploadStatus::ScanFailed
        )
    }

    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}
