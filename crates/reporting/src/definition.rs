//! Report definitions (docs/15 §9).
//!
//! ReportDefinition: dataset query, sections/tables/charts/summaries,
//! locale/timezone, template version, field formatting, confidentiality/
//! watermark, page/row limits, allowed audiences/formats, snapshot
//! consistency, schedule/delivery.

use serde::{Deserialize, Serialize};

/// Export format (docs/15 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    /// XLSX format.
    Xlsx,
    /// CSV format.
    Csv,
    /// NDJSON format.
    Ndjson,
    /// JSON package format.
    Json,
    /// DOCX format.
    Docx,
    /// PDF format.
    Pdf,
}

impl ExportFormat {
    /// Get file extension.
    pub fn extension(self) -> &'static str {
        match self {
            Self::Xlsx => "xlsx",
            Self::Csv => "csv",
            Self::Ndjson => "ndjson",
            Self::Json => "json",
            Self::Docx => "docx",
            Self::Pdf => "pdf",
        }
    }

    /// Get MIME type.
    pub fn mime_type(self) -> &'static str {
        match self {
            Self::Xlsx => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::Csv => "text/csv",
            Self::Ndjson => "application/x-ndjson",
            Self::Json => "application/json",
            Self::Docx => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Pdf => "application/pdf",
        }
    }
}

/// Report status (docs/15 §9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportStatus {
    /// Report definition is draft.
    Draft,
    /// Report definition is active.
    Active,
    /// Report generation is scheduled.
    Scheduled,
    /// Report is being generated.
    Generating,
    /// Report is ready.
    Ready,
    /// Report generation failed.
    Failed,
    /// Report is retired.
    Retired,
}

/// Allowed audience for a report (docs/15 §9, §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportAudience {
    /// Audience type: "user", "group", "role".
    pub audience_type: String,
    /// Audience ID (user ID, group ID, or role name).
    pub audience_id: String,
    /// Whether this audience can re-export.
    pub can_reexport: bool,
}

/// A table definition in a report section (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableDefinition {
    /// Table name (machine name).
    pub name: String,
    /// Display label key (i18n).
    pub label_key: String,
    /// Dataset ID (source data).
    pub dataset_id: String,
    /// Columns to include (empty = all).
    pub columns: Vec<String>,
    /// Row limit for this table.
    pub row_limit: Option<u32>,
    /// Whether to show totals.
    pub show_totals: bool,
}

/// A chart definition in a report section (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChartDefinition {
    /// Chart name.
    pub name: String,
    /// Display label key (i18n).
    pub label_key: String,
    /// Chart type: "bar", "line", "pie", "scatter", "table".
    pub chart_type: String,
    /// Dataset ID (source data).
    pub dataset_id: String,
    /// X-axis column.
    pub x_column: String,
    /// Y-axis column(s).
    pub y_columns: Vec<String>,
    /// Group by column (optional).
    pub group_by: Option<String>,
}

/// A summary definition in a report section (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SummaryDefinition {
    /// Summary name.
    pub name: String,
    /// Display label key (i18n).
    pub label_key: String,
    /// Dataset ID (source data).
    pub dataset_id: String,
    /// Aggregation function: "count", "sum", "avg", "min", "max".
    pub aggregation: String,
    /// Column to aggregate (empty for count).
    pub column: Option<String>,
    /// Filter expression (optional).
    pub filter: Option<String>,
}

/// A report section (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportSection {
    /// Section name.
    pub name: String,
    /// Display label key (i18n).
    pub label_key: String,
    /// Section order.
    pub order: u32,
    /// Tables in this section.
    pub tables: Vec<TableDefinition>,
    /// Charts in this section.
    pub charts: Vec<ChartDefinition>,
    /// Summaries in this section.
    pub summaries: Vec<SummaryDefinition>,
}

/// A report definition (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReportDefinition {
    /// Report definition ID.
    pub definition_id: String,
    /// Report version.
    pub version: String,
    /// Report name.
    pub name: String,
    /// Report description key (i18n).
    pub description_key: String,
    /// Dataset IDs used by this report.
    pub dataset_ids: Vec<String>,
    /// Sections.
    pub sections: Vec<ReportSection>,
    /// Default locale (e.g. "ru", "en").
    pub default_locale: String,
    /// Default timezone (IANA).
    pub default_timezone: String,
    /// Template ID (DOCX/XLSX/PDF template version).
    pub template_id: String,
    /// Template version.
    pub template_version: String,
    /// Allowed export formats.
    pub allowed_formats: Vec<ExportFormat>,
    /// Page limit.
    pub page_limit: Option<u32>,
    /// Row limit (total across all tables).
    pub row_limit: Option<u32>,
    /// Confidentiality level: "public", "restricted", "confidential".
    pub confidentiality: String,
    /// Whether to apply watermark.
    pub watermark: bool,
    /// Allowed audiences (docs/15 §9, §12).
    pub allowed_audiences: Vec<ReportAudience>,
    /// Whether snapshot consistency is required.
    pub requires_snapshot: bool,
    /// Status.
    pub status: ReportStatus,
    /// Owner (team or user).
    pub owner: String,
    /// Schedule (cron expression, if scheduled).
    pub schedule: Option<String>,
}

impl ReportDefinition {
    /// Check if a format is allowed.
    pub fn is_format_allowed(&self, format: ExportFormat) -> bool {
        self.allowed_formats.contains(&format)
    }

    /// Check if an audience is allowed.
    pub fn is_audience_allowed(&self, audience_type: &str, audience_id: &str) -> bool {
        self.allowed_audiences
            .iter()
            .any(|a| a.audience_type == audience_type && a.audience_id == audience_id)
    }

    /// Get all tables across all sections.
    pub fn all_tables(&self) -> Vec<&TableDefinition> {
        self.sections.iter().flat_map(|s| s.tables.iter()).collect()
    }

    /// Get all charts across all sections.
    pub fn all_charts(&self) -> Vec<&ChartDefinition> {
        self.sections.iter().flat_map(|s| s.charts.iter()).collect()
    }

    /// Get all summaries across all sections.
    pub fn all_summaries(&self) -> Vec<&SummaryDefinition> {
        self.sections
            .iter()
            .flat_map(|s| s.summaries.iter())
            .collect()
    }
}
