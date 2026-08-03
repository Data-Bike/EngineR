//! Risk factor (docs/14 §9).
//!
//! RiskFactor — entity/statement set produced when a criterion triggers.
//! Repeated evaluation upserts by target+criterion+evidence window, not
//! creating millions of identical factors (docs/14 §9).

use crate::criterion::CriterionSeverity;
use serde::{Deserialize, Serialize};

/// Status of a risk factor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorStatus {
    /// Factor is active (criterion triggered).
    Active,
    /// Factor has been resolved (condition no longer met).
    Resolved,
    /// Factor has been suppressed by an operator.
    Suppressed,
    /// Factor has been overridden by an operator.
    Overridden,
    /// Factor has expired (evidence window passed).
    Expired,
}

/// Suppression/override record (docs/14 §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suppression {
    /// Actor who suppressed/overrode.
    pub actor: String,
    /// Reason for suppression/override.
    pub reason: String,
    /// When the suppression was applied.
    pub applied_at: chrono::DateTime<chrono::Utc>,
    /// When the suppression expires (None = indefinite).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this is a suppression or override.
    pub kind: SuppressionKind,
}

/// Kind of suppression.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuppressionKind {
    /// Suppress: factor is ignored but not resolved.
    Suppress,
    /// Override: factor is replaced with a manual assessment.
    Override,
}

/// A risk factor (docs/14 §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Factor {
    /// Factor ID (UUID).
    pub factor_id: String,
    /// Criterion ID that produced this factor.
    pub criterion_id: String,
    /// Criterion version.
    pub criterion_version: String,
    /// Target entity public ID.
    pub target_entity_id: String,
    /// When the factor was detected.
    pub detected_at: chrono::DateTime<chrono::Utc>,
    /// When the factor was resolved (if status is Resolved).
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Severity (copied from criterion at detection time).
    pub severity: CriterionSeverity,
    /// Score contribution (decimal string, e.g. "25.0").
    pub score_contribution: String,
    /// Status.
    pub status: FactorStatus,
    /// Evidence statement/source references.
    pub evidence: Vec<EvidenceRef>,
    /// Explanation parameters (for i18n).
    pub explanation_params: serde_json::Value,
    /// Suppression/override record (if any).
    pub suppression: Option<Suppression>,
    /// Dedup key: target + criterion + evidence window.
    pub dedup_key: String,
}

/// Evidence reference (statement or observation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// "statement" or "observation".
    pub kind: String,
    /// Reference ID (statement ID or observation ID).
    pub ref_id: String,
    /// Source connector type ID (for observations).
    pub source_connector_type_id: Option<String>,
    /// Content hash (for observations).
    pub content_hash: Option<String>,
}

impl Factor {
    /// Create a new active factor.
    pub fn new(
        criterion_id: &str,
        criterion_version: &str,
        target_entity_id: &str,
        severity: CriterionSeverity,
        score_contribution: &str,
        evidence: Vec<EvidenceRef>,
        explanation_params: serde_json::Value,
    ) -> Self {
        let detected_at = chrono::Utc::now();
        let dedup_key = format!(
            "{}:{}:{}:{}",
            target_entity_id, criterion_id, criterion_version,
            detected_at.format("%Y-%m-%d")
        );
        Self {
            factor_id: uuid::Uuid::now_v7().to_string(),
            criterion_id: criterion_id.to_string(),
            criterion_version: criterion_version.to_string(),
            target_entity_id: target_entity_id.to_string(),
            detected_at,
            resolved_at: None,
            severity,
            score_contribution: score_contribution.to_string(),
            status: FactorStatus::Active,
            evidence,
            explanation_params,
            suppression: None,
            dedup_key,
        }
    }

    /// Resolve the factor.
    pub fn resolve(&mut self) {
        self.status = FactorStatus::Resolved;
        self.resolved_at = Some(chrono::Utc::now());
    }

    /// Suppress the factor.
    pub fn suppress(&mut self, suppression: Suppression) {
        self.suppression = Some(suppression);
        self.status = FactorStatus::Suppressed;
    }

    /// Check if the factor is currently active (not resolved/suppressed/expired).
    pub fn is_active(&self) -> bool {
        matches!(self.status, FactorStatus::Active | FactorStatus::Overridden)
    }

    /// Check if the factor matches a dedup key.
    pub fn matches_dedup(&self, dedup_key: &str) -> bool {
        self.dedup_key == dedup_key
    }
}
