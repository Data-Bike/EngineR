//! Template definitions (docs/15 §9).
//!
//! Template does not contain SQL and cannot access arbitrary object path
//! outside declared dataset (docs/15 §9). Templates are versioned with
//! locale-specific variants.

use serde::{Deserialize, Serialize};

/// Template format (docs/15 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateFormat {
    /// DOCX template.
    Docx,
    /// XLSX template.
    Xlsx,
    /// PDF template.
    Pdf,
    /// HTML template (for screen rendering).
    Html,
}

/// Template locale variant (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateLocale {
    /// Locale code (e.g. "ru", "en", "ar").
    pub locale: String,
    /// Template storage URL (for this locale).
    pub storage_url: String,
    /// Template content hash.
    pub content_hash: String,
}

/// Field formatting rules (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldFormat {
    /// Field name (matches dataset column name).
    pub field_name: String,
    /// Format pattern (date/number format).
    pub pattern: String,
    /// Whether to apply masking (for classified fields).
    pub masked: bool,
    /// Mask pattern (if masked).
    pub mask_pattern: Option<String>,
}

/// A template version (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateVersion {
    /// Version string.
    pub version: String,
    /// Template format.
    pub format: TemplateFormat,
    /// When this version was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Whether this is the current version.
    pub current: bool,
    /// Locale variants.
    pub locales: Vec<TemplateLocale>,
    /// Field formatting rules.
    pub field_formats: Vec<FieldFormat>,
}

/// A template definition (docs/15 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateDefinition {
    /// Template ID.
    pub template_id: String,
    /// Template name.
    pub name: String,
    /// Template description key (i18n).
    pub description_key: String,
    /// Template format.
    pub format: TemplateFormat,
    /// Versions (history).
    pub versions: Vec<TemplateVersion>,
    /// Allowed datasets (templates can only access declared datasets).
    pub allowed_datasets: Vec<String>,
    /// Whether the template contains a watermark placeholder.
    pub has_watermark: bool,
}

impl TemplateDefinition {
    /// Get the current version.
    pub fn current_version(&self) -> Option<&TemplateVersion> {
        self.versions.iter().find(|v| v.current)
    }

    /// Get a specific version.
    pub fn get_version(&self, version: &str) -> Option<&TemplateVersion> {
        self.versions.iter().find(|v| v.version == version)
    }

    /// Get the locale variant for the current version.
    pub fn current_locale(&self, locale: &str) -> Option<&TemplateLocale> {
        self.current_version()
            .and_then(|v| v.locales.iter().find(|l| l.locale == locale))
    }

    /// Check if a dataset is allowed.
    pub fn is_dataset_allowed(&self, dataset_id: &str) -> bool {
        self.allowed_datasets.contains(&dataset_id.to_string())
    }

    /// Add a new version (marks previous current as non-current).
    pub fn add_version(&mut self, mut version: TemplateVersion) {
        for v in &mut self.versions {
            v.current = false;
        }
        version.current = true;
        self.versions.push(version);
    }
}

/// Convenience alias.
pub type Template = TemplateDefinition;
