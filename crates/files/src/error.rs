//! File errors.

use thiserror::Error;

pub type FileResult<T> = Result<T, FileError>;

#[derive(Debug, Clone, PartialEq, Eq, Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum FileError {
    #[error("upload: {message}")]
    Upload { message: String },

    #[error("scan: {message}")]
    Scan { message: String },

    #[error("storage: {message}")]
    Storage { message: String },

    #[error("not_found: {message}")]
    NotFound { message: String },

    #[error("size_exceeded: {max_bytes} bytes")]
    SizeExceeded { max_bytes: u64 },

    #[error("scan_failed: {message}")]
    ScanFailed { message: String },

    #[error("scan_rejected: {findings:?}")]
    ScanRejected { findings: Vec<String> },

    #[error("invalid_config: {message}")]
    InvalidConfig { message: String },

    #[error("checksum_mismatch: expected {expected}, got {got}")]
    ChecksumMismatch { expected: String, got: String },
}
