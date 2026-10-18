//! Files: staged upload, object storage abstraction, scanning
//! (docs/15 §1-2).
//!
//! Files are untrusted: size/zip-bomb/formula/macro/external-link/MIME
//! checks and antivirus (docs/15 §2). Upload is separated from processing
//! (docs/15 §1). Input file is immutable and has hash.

#![forbid(unsafe_code)]

pub mod error;
pub mod scan;
pub mod storage;
pub mod upload;

pub use error::{FileError, FileResult};
pub use scan::{
    FileScanResult, FileScanStatus, FileScanner, FileSignature, ScanConfig as FileScanConfig,
    ScanFinding, ScanSeverity,
};
pub use storage::{ObjectMetadata, ObjectStorage, StorageClass, StorageConfig, StorageUrl};
pub use upload::{
    StagedUpload, UploadAbort, UploadChunk, UploadComplete, UploadInit, UploadStatus,
};
