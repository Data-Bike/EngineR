//! File scanning (docs/15 §2).
//!
//! Files are untrusted: size/zip-bomb/formula/macro/external-link/MIME
//! checks and antivirus.

use serde::{Deserialize, Serialize};

/// Scan severity (docs/15 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanSeverity {
    /// Informational, no action needed.
    Info,
    /// Warning, may require review.
    Warning,
    /// Critical, file rejected.
    Critical,
}

/// Scan status (docs/15 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileScanStatus {
    /// Scan pending.
    Pending,
    /// Scan in progress.
    Scanning,
    /// Scan passed, file is clean.
    Clean,
    /// Scan found issues but file is accepted with warnings.
    AcceptedWithWarnings,
    /// Scan rejected the file.
    Rejected,
    /// Scan failed (scanner error).
    Failed,
}

/// A scan finding (docs/15 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanFinding {
    /// Finding kind: "zip_bomb", "formula", "macro", "external_link",
    /// "mime_mismatch", "virus", "oversized", "embedded_object".
    pub kind: String,
    /// Severity.
    pub severity: ScanSeverity,
    /// Human-readable message.
    pub message: String,
    /// Location (sheet/cell/offset, if applicable).
    pub location: Option<String>,
}

/// File signature (magic bytes detection, docs/15 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSignature {
    /// Detected format (e.g. "xlsx", "csv", "ndjson", "pdf", "docx").
    pub format: String,
    /// Detected MIME type.
    pub mime_type: String,
    /// Whether the signature matches the declared content type.
    pub matches_declared: bool,
}

/// Scan configuration (docs/15 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Maximum file size in bytes.
    pub max_size_bytes: u64,
    /// Maximum decompressed size (for zip-bomb detection).
    pub max_decompressed_size: u64,
    /// Whether to check for formulas.
    pub check_formulas: bool,
    /// Whether to check for macros.
    pub check_macros: bool,
    /// Whether to check for external links.
    pub check_external_links: bool,
    /// Whether to run antivirus scan.
    pub run_antivirus: bool,
    /// Whether to verify MIME type matches content.
    pub verify_mime: bool,
    /// Allowed formats (empty = all allowed).
    pub allowed_formats: Vec<String>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            max_size_bytes: 100 * 1024 * 1024,        // 100 MB
            max_decompressed_size: 500 * 1024 * 1024, // 500 MB
            check_formulas: true,
            check_macros: true,
            check_external_links: true,
            run_antivirus: true,
            verify_mime: true,
            allowed_formats: vec![
                "xlsx".to_string(),
                "csv".to_string(),
                "tsv".to_string(),
                "ndjson".to_string(),
                "json".to_string(),
                "pdf".to_string(),
                "docx".to_string(),
            ],
        }
    }
}

/// File scan result (docs/15 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileScanResult {
    /// Scan ID (UUID).
    pub scan_id: String,
    /// Object ID being scanned.
    pub object_id: String,
    /// Scan status.
    pub status: FileScanStatus,
    /// File signature (detected format).
    pub signature: Option<FileSignature>,
    /// Findings (warnings/critical issues).
    pub findings: Vec<ScanFinding>,
    /// Scanned at.
    pub scanned_at: chrono::DateTime<chrono::Utc>,
    /// Scanner version.
    pub scanner_version: String,
    /// Duration in milliseconds.
    pub duration_ms: u64,
}

impl FileScanResult {
    /// Create a new pending scan result.
    pub fn pending(object_id: &str) -> Self {
        Self {
            scan_id: uuid::Uuid::now_v7().to_string(),
            object_id: object_id.to_string(),
            status: FileScanStatus::Pending,
            signature: None,
            findings: Vec::new(),
            scanned_at: chrono::Utc::now(),
            scanner_version: "1.0.0".to_string(),
            duration_ms: 0,
        }
    }

    /// Check if the scan is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            FileScanStatus::Clean
                | FileScanStatus::AcceptedWithWarnings
                | FileScanStatus::Rejected
                | FileScanStatus::Failed
        )
    }

    /// Check if the file passed scanning (clean or accepted with warnings).
    pub fn passed(&self) -> bool {
        matches!(
            self.status,
            FileScanStatus::Clean | FileScanStatus::AcceptedWithWarnings
        )
    }

    /// Get critical findings.
    pub fn critical_findings(&self) -> Vec<&ScanFinding> {
        self.findings
            .iter()
            .filter(|f| f.severity == ScanSeverity::Critical)
            .collect()
    }

    /// Mark scan as clean.
    pub fn mark_clean(&mut self, signature: FileSignature, duration_ms: u64) {
        self.status = FileScanStatus::Clean;
        self.signature = Some(signature);
        self.scanned_at = chrono::Utc::now();
        self.duration_ms = duration_ms;
    }

    /// Mark scan as accepted with warnings.
    pub fn mark_accepted_with_warnings(
        &mut self,
        signature: FileSignature,
        findings: Vec<ScanFinding>,
        duration_ms: u64,
    ) {
        self.status = FileScanStatus::AcceptedWithWarnings;
        self.signature = Some(signature);
        self.findings = findings;
        self.scanned_at = chrono::Utc::now();
        self.duration_ms = duration_ms;
    }

    /// Mark scan as rejected.
    pub fn mark_rejected(
        &mut self,
        signature: Option<FileSignature>,
        findings: Vec<ScanFinding>,
        duration_ms: u64,
    ) {
        self.status = FileScanStatus::Rejected;
        self.signature = signature;
        self.findings = findings;
        self.scanned_at = chrono::Utc::now();
        self.duration_ms = duration_ms;
    }

    /// Mark scan as failed (scanner error).
    pub fn mark_failed(&mut self, error: &str, duration_ms: u64) {
        self.status = FileScanStatus::Failed;
        self.findings.push(ScanFinding {
            kind: "scanner_error".to_string(),
            severity: ScanSeverity::Critical,
            message: error.to_string(),
            location: None,
        });
        self.scanned_at = chrono::Utc::now();
        self.duration_ms = duration_ms;
    }
}

/// File scanner trait (docs/15 §2).
///
/// Implementations provide format detection and security scanning.
pub trait FileScanner: Send + Sync {
    /// Detect file format from magic bytes.
    fn detect_format(&self, magic_bytes: &[u8]) -> FileSignature;

    /// Scan a file for security issues.
    fn scan(&self, object_id: &str, config: &ScanConfig) -> FileScanResult;
}
