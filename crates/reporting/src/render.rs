//! Template rendering (docs/15 §8-9).
//!
//! Runtime: reauthorize, bind policy/schema snapshot, stream chunks,
//! apply masks before writer, escape formula injection in CSV/XLSX,
//! watermark/classification, encrypt/sign if required, artifact TTL/
//! download decision (docs/15 §8).

use crate::definition::ExportFormat;
use crate::error::ReportResult;
use serde::{Deserialize, Serialize};

/// Render status (docs/15 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderStatus {
    /// Render is pending.
    Pending,
    /// Render is in progress.
    Rendering,
    /// Render completed successfully.
    Completed,
    /// Render failed.
    Failed,
}

/// Render context (docs/15 §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderContext {
    /// Tenant ID.
    pub tenant_id: String,
    /// Actor requesting the render.
    pub actor: String,
    /// Locale (e.g. "ru", "en").
    pub locale: String,
    /// Timezone (IANA).
    pub timezone: String,
    /// Export format.
    pub format: ExportFormat,
    /// Whether this is a scheduled render (service principal).
    pub scheduled: bool,
    /// Snapshot grant ID (if using approved snapshot grant, docs/15 §12).
    pub snapshot_grant_id: Option<String>,
    /// Whether to apply watermark.
    pub watermark: bool,
    /// Whether to encrypt the output.
    pub encrypt: bool,
    /// Whether to sign the output.
    pub sign: bool,
    /// Classification level.
    pub classification: String,
}

/// Render output (docs/15 §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderOutput {
    /// Output ID (UUID).
    pub output_id: String,
    /// Report definition ID.
    pub definition_id: String,
    /// Report version.
    pub definition_version: String,
    /// Template version used.
    pub template_version: String,
    /// Schema revision pinned.
    pub schema_revision_id: String,
    /// Policy revision pinned.
    pub policy_revision_id: String,
    /// Export format.
    pub format: ExportFormat,
    /// Output storage URL.
    pub storage_url: String,
    /// Content hash.
    pub content_hash: String,
    /// Size in bytes.
    pub size: u64,
    /// Page count (if applicable).
    pub page_count: Option<u32>,
    /// Row count (total across all tables).
    pub row_count: u64,
    /// Rendered at.
    pub rendered_at: chrono::DateTime<chrono::Utc>,
    /// Duration in milliseconds.
    pub duration_ms: u64,
    /// Status.
    pub status: RenderStatus,
    /// Whether the output is encrypted.
    pub encrypted: bool,
    /// Whether the output is signed.
    pub signed: bool,
    /// Artifact TTL in days.
    pub ttl_days: Option<u32>,
}

/// Render result (returned by the renderer).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderResult {
    /// Output metadata.
    pub output: RenderOutput,
    /// Warnings (non-fatal issues).
    pub warnings: Vec<String>,
}

impl RenderOutput {
    /// Create a new pending render output.
    pub fn pending(
        definition_id: &str,
        definition_version: &str,
        template_version: &str,
        schema_revision_id: &str,
        policy_revision_id: &str,
        format: ExportFormat,
    ) -> Self {
        Self {
            output_id: uuid::Uuid::now_v7().to_string(),
            definition_id: definition_id.to_string(),
            definition_version: definition_version.to_string(),
            template_version: template_version.to_string(),
            schema_revision_id: schema_revision_id.to_string(),
            policy_revision_id: policy_revision_id.to_string(),
            format,
            storage_url: String::new(),
            content_hash: String::new(),
            size: 0,
            page_count: None,
            row_count: 0,
            rendered_at: chrono::Utc::now(),
            duration_ms: 0,
            status: RenderStatus::Pending,
            encrypted: false,
            signed: false,
            ttl_days: None,
        }
    }

    /// Mark as completed.
    pub fn mark_completed(
        &mut self,
        storage_url: &str,
        content_hash: &str,
        size: u64,
        page_count: Option<u32>,
        row_count: u64,
        duration_ms: u64,
    ) {
        self.status = RenderStatus::Completed;
        self.storage_url = storage_url.to_string();
        self.content_hash = content_hash.to_string();
        self.size = size;
        self.page_count = page_count;
        self.row_count = row_count;
        self.rendered_at = chrono::Utc::now();
        self.duration_ms = duration_ms;
    }

    /// Mark as failed.
    pub fn mark_failed(&mut self) {
        self.status = RenderStatus::Failed;
    }

    /// Check if the render is terminal.
    pub fn is_terminal(&self) -> bool {
        matches!(self.status, RenderStatus::Completed | RenderStatus::Failed)
    }
}

/// Template renderer trait (docs/15 §8-9).
///
/// Implementations render report definitions into output files using
/// templates. The renderer must:
/// - reauthorize on every chunk (docs/15 §12);
/// - apply masks before writer (docs/15 §8);
/// - escape formula injection in CSV/XLSX (docs/15 §8);
/// - apply watermark/classification (docs/15 §8);
/// - encrypt/sign if required (docs/15 §8).
pub trait TemplateRenderer: Send + Sync {
    /// Render a report definition.
    fn render(
        &self,
        definition: &crate::definition::ReportDefinition,
        context: &RenderContext,
    ) -> ReportResult<RenderResult>;
}

/// Escape formula injection in CSV/XLSX text context (docs/15 §8).
///
/// Any value starting with =, +, -, @ is escaped to prevent formula injection.
pub fn escape_formula_injection(value: &str) -> String {
    let trimmed = value.trim_start();
    if trimmed.starts_with('=')
        || trimmed.starts_with('+')
        || trimmed.starts_with('-')
        || trimmed.starts_with('@')
    {
        format!("'{}", value)
    } else {
        value.to_string()
    }
}
