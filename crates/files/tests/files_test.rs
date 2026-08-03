//! Files crate tests (docs/15 §1-2).

#![cfg(test)]

use enginer_files::{
    FileScanResult, FileScanStatus, FileSignature, FileScanConfig,
    ScanFinding, ScanSeverity, StagedUpload, UploadInit, UploadStatus,
};
use std::collections::HashMap;

fn make_upload_init(size: u64, chunk_size: u64) -> UploadInit {
    UploadInit {
        tenant_id: "tenant-001".to_string(),
        filename: "test.xlsx".to_string(),
        content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        expected_size: size,
        expected_hash: "abc123".to_string(),
        hash_algorithm: "blake2b".to_string(),
        chunk_size,
        metadata: HashMap::new(),
        storage_class: "hot".to_string(),
    }
}

#[test]
fn staged_upload_init() {
    let init = make_upload_init(1000, 100);
    let upload = StagedUpload::init(init).unwrap();
    assert_eq!(upload.status, UploadStatus::Initiated);
    assert_eq!(upload.total_chunks, 10);
    assert!(!upload.is_terminal());
}

#[test]
fn staged_upload_init_zero_size_fails() {
    let init = make_upload_init(0, 100);
    assert!(StagedUpload::init(init).is_err());
}

#[test]
fn staged_upload_init_zero_chunk_fails() {
    let init = make_upload_init(1000, 0);
    assert!(StagedUpload::init(init).is_err());
}

#[test]
fn staged_upload_receive_chunks() {
    let init = make_upload_init(300, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    assert_eq!(upload.total_chunks, 3);

    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    assert_eq!(upload.status, UploadStatus::Uploading);

    upload.receive_chunk(enginer_files::UploadChunk {
        index: 1,
        size: 100,
        hash: "h1".to_string(),
        is_last: false,
    }).unwrap();

    upload.receive_chunk(enginer_files::UploadChunk {
        index: 2,
        size: 100,
        hash: "h2".to_string(),
        is_last: true,
    }).unwrap();
    assert_eq!(upload.status, UploadStatus::AllChunksReceived);
    assert!(upload.is_complete());
}

#[test]
fn staged_upload_idempotent_chunk() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    let chunk = enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    };
    upload.receive_chunk(chunk.clone()).unwrap();
    // Re-upload same chunk with same hash — should be OK (idempotent)
    upload.receive_chunk(chunk).unwrap();
    assert_eq!(upload.received_chunks.len(), 1);
}

#[test]
fn staged_upload_chunk_hash_mismatch() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    let result = upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "different".to_string(),
        is_last: false,
    });
    assert!(result.is_err());
}

#[test]
fn staged_upload_chunk_out_of_range() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    let result = upload.receive_chunk(enginer_files::UploadChunk {
        index: 5,
        size: 100,
        hash: "h5".to_string(),
        is_last: true,
    });
    assert!(result.is_err());
}

#[test]
fn staged_upload_complete() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 1,
        size: 100,
        hash: "h1".to_string(),
        is_last: true,
    }).unwrap();
    upload.complete(enginer_files::UploadComplete {
        final_hash: "abc123".to_string(),
        total_size: 200,
    }).unwrap();
    assert_eq!(upload.status, UploadStatus::Completed);
    assert!(upload.is_terminal());
    assert!(upload.object_id.is_some());
    assert_eq!(upload.final_hash.as_deref(), Some("abc123"));
}

#[test]
fn staged_upload_complete_wrong_hash() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 1,
        size: 100,
        hash: "h1".to_string(),
        is_last: true,
    }).unwrap();
    let result = upload.complete(enginer_files::UploadComplete {
        final_hash: "wrong".to_string(),
        total_size: 200,
    });
    assert!(result.is_err());
}

#[test]
fn staged_upload_abort() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    upload.abort(enginer_files::UploadAbort {
        reason: "user cancelled".to_string(),
    }).unwrap();
    assert_eq!(upload.status, UploadStatus::Aborted);
    assert!(upload.is_terminal());
    assert!(upload.received_chunks.is_empty());
}

#[test]
fn staged_upload_abort_completed_fails() {
    let init = make_upload_init(200, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 1,
        size: 100,
        hash: "h1".to_string(),
        is_last: true,
    }).unwrap();
    upload.complete(enginer_files::UploadComplete {
        final_hash: "abc123".to_string(),
        total_size: 200,
    }).unwrap();
    let result = upload.abort(enginer_files::UploadAbort {
        reason: "test".to_string(),
    });
    assert!(result.is_err());
}

#[test]
fn staged_upload_progress() {
    let init = make_upload_init(400, 100);
    let mut upload = StagedUpload::init(init).unwrap();
    assert_eq!(upload.progress(), 0.0);
    upload.receive_chunk(enginer_files::UploadChunk {
        index: 0,
        size: 100,
        hash: "h0".to_string(),
        is_last: false,
    }).unwrap();
    assert_eq!(upload.progress(), 0.25);
}

// --- Scan tests ---

#[test]
fn scan_result_pending() {
    let result = FileScanResult::pending("obj-001");
    assert_eq!(result.status, FileScanStatus::Pending);
    assert!(!result.is_terminal());
}

#[test]
fn scan_result_mark_clean() {
    let mut result = FileScanResult::pending("obj-001");
    let sig = FileSignature {
        format: "xlsx".to_string(),
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        matches_declared: true,
    };
    result.mark_clean(sig, 150);
    assert_eq!(result.status, FileScanStatus::Clean);
    assert!(result.passed());
    assert!(result.is_terminal());
}

#[test]
fn scan_result_mark_rejected() {
    let mut result = FileScanResult::pending("obj-001");
    let findings = vec![
        ScanFinding {
            kind: "macro".to_string(),
            severity: ScanSeverity::Critical,
            message: "VBA macro detected".to_string(),
            location: Some("Sheet1!A1".to_string()),
        },
    ];
    result.mark_rejected(None, findings, 200);
    assert_eq!(result.status, FileScanStatus::Rejected);
    assert!(!result.passed());
    assert!(result.is_terminal());
    assert_eq!(result.critical_findings().len(), 1);
}

#[test]
fn scan_result_mark_accepted_with_warnings() {
    let mut result = FileScanResult::pending("obj-001");
    let sig = FileSignature {
        format: "xlsx".to_string(),
        mime_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
        matches_declared: true,
    };
    let findings = vec![
        ScanFinding {
            kind: "formula".to_string(),
            severity: ScanSeverity::Warning,
            message: "Formula detected in cell".to_string(),
            location: Some("Sheet1!B2".to_string()),
        },
    ];
    result.mark_accepted_with_warnings(sig, findings, 100);
    assert_eq!(result.status, FileScanStatus::AcceptedWithWarnings);
    assert!(result.passed());
    assert!(result.is_terminal());
}

#[test]
fn scan_result_mark_failed() {
    let mut result = FileScanResult::pending("obj-001");
    result.mark_failed("scanner timeout", 5000);
    assert_eq!(result.status, FileScanStatus::Failed);
    assert!(!result.passed());
    assert!(result.is_terminal());
    assert_eq!(result.critical_findings().len(), 1);
}

#[test]
fn scan_config_default() {
    let config = FileScanConfig::default();
    assert_eq!(config.max_size_bytes, 100 * 1024 * 1024);
    assert!(config.check_formulas);
    assert!(config.check_macros);
    assert!(config.run_antivirus);
}

// --- Storage tests ---

#[test]
fn storage_class_from_str() {
    use enginer_files::storage::StorageClass;
    assert_eq!(StorageClass::from_str("hot"), Some(StorageClass::Hot));
    assert_eq!(StorageClass::from_str("COOL"), Some(StorageClass::Cool));
    assert_eq!(StorageClass::from_str("archive"), Some(StorageClass::Archive));
    assert_eq!(StorageClass::from_str("unknown"), None);
}

#[test]
fn object_metadata_new() {
    use enginer_files::storage::{ObjectMetadata, StorageClass, StorageUrl};
    let storage_url = StorageUrl {
        url: "s3://bucket/key".to_string(),
        object_id: "obj-001".to_string(),
    };
    let meta = ObjectMetadata::new(
        "tenant-001",
        "hash123",
        "blake2b",
        1024,
        "application/pdf",
        "report.pdf",
        StorageClass::Hot,
        storage_url,
        "restricted",
    );
    assert_eq!(meta.tenant_id, "tenant-001");
    assert_eq!(meta.size, 1024);
    assert!(!meta.legal_hold);
    assert!(!meta.is_expired());
}

#[test]
fn object_metadata_ttl_expiry() {
    use enginer_files::storage::{ObjectMetadata, StorageClass, StorageUrl};
    let storage_url = StorageUrl {
        url: "s3://bucket/key".to_string(),
        object_id: "obj-001".to_string(),
    };
    let meta = ObjectMetadata::new(
        "tenant-001",
        "hash123",
        "blake2b",
        1024,
        "application/pdf",
        "report.pdf",
        StorageClass::Hot,
        storage_url,
        "restricted",
    )
    .with_ttl(0); // expires immediately
    assert!(meta.is_expired());
}

#[test]
fn object_metadata_legal_hold_prevents_expiry() {
    use enginer_files::storage::{ObjectMetadata, StorageClass, StorageUrl};
    let storage_url = StorageUrl {
        url: "s3://bucket/key".to_string(),
        object_id: "obj-001".to_string(),
    };
    let mut meta = ObjectMetadata::new(
        "tenant-001",
        "hash123",
        "blake2b",
        1024,
        "application/pdf",
        "report.pdf",
        StorageClass::Hot,
        storage_url,
        "restricted",
    )
    .with_ttl(0);
    meta.apply_legal_hold();
    assert!(!meta.is_expired());
    meta.release_legal_hold();
    assert!(meta.is_expired());
}

#[test]
fn object_metadata_add_metadata() {
    use enginer_files::storage::{ObjectMetadata, StorageClass, StorageUrl};
    let storage_url = StorageUrl {
        url: "s3://bucket/key".to_string(),
        object_id: "obj-001".to_string(),
    };
    let mut meta = ObjectMetadata::new(
        "tenant-001",
        "hash123",
        "blake2b",
        1024,
        "application/pdf",
        "report.pdf",
        StorageClass::Hot,
        storage_url,
        "restricted",
    );
    meta.add_metadata("source", "import");
    meta.add_metadata("job_id", "job-001");
    assert_eq!(meta.user_metadata.len(), 2);
}
