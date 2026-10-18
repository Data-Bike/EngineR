//! Schema validation report (docs/12 §2).

use serde::{Deserialize, Serialize};

/// Severity of a validation finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationFinding {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    /// Optional path to the offending element.
    pub path: Option<String>,
}

/// Result of validating a schema draft.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub findings: Vec<ValidationFinding>,
}

impl ValidationReport {
    /// Empty report.
    pub fn empty() -> Self {
        Self {
            findings: Vec::new(),
        }
    }

    /// True if there are no error-severity findings.
    pub fn passes(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Error)
    }

    /// Add a finding.
    pub fn add(
        &mut self,
        severity: Severity,
        code: impl Into<String>,
        message: impl Into<String>,
        path: Option<String>,
    ) {
        self.findings.push(ValidationFinding {
            severity,
            code: code.into(),
            message: message.into(),
            path,
        });
    }
}
