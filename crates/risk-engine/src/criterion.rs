//! Risk criterion definition (docs/14 §8).
//!
//! Each criterion has ID/version, input predicates, severity, weight,
//! explanation key, evidence selector, effective interval, owner, tests.

use crate::ast::Expr;
use serde::{Deserialize, Serialize};

/// Severity of a criterion (docs/14 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionSeverity {
    /// Informational, no risk contribution.
    Info,
    /// Low severity.
    Low,
    /// Medium severity.
    Medium,
    /// High severity.
    High,
    /// Critical severity (blocking).
    Critical,
}

impl CriterionSeverity {
    /// Get the numeric weight contribution (docs/14 §8).
    pub fn weight_contribution(self) -> u32 {
        match self {
            Self::Info => 0,
            Self::Low => 10,
            Self::Medium => 25,
            Self::High => 50,
            Self::Critical => 100,
        }
    }
}

/// Status of a criterion definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CriterionStatus {
    /// Draft, not yet active.
    Draft,
    /// Active and being evaluated.
    Active,
    /// Retired, no longer evaluated.
    Retired,
}

/// A risk criterion definition (docs/14 §8).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Criterion {
    /// Stable criterion ID.
    pub criterion_id: String,
    /// Criterion version.
    pub version: String,
    /// Human-readable name.
    pub name: String,
    /// The typed expression to evaluate.
    pub expression: Expr,
    /// Severity level.
    pub severity: CriterionSeverity,
    /// Weight (0.0 - 1.0, stored as string for decimal precision).
    pub weight: String,
    /// Explanation key (i18n key for the explanation text).
    pub explanation_key: String,
    /// Evidence selector: which observations to include as evidence.
    pub evidence_selector: EvidenceSelector,
    /// Effective interval (valid from / valid to).
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
    /// Owner (team or user responsible).
    pub owner: String,
    /// Status.
    pub status: CriterionStatus,
    /// Test cases (criterion must pass its own tests before activation).
    pub tests: Vec<CriterionTest>,
}

/// Evidence selector for a criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceSelector {
    /// Which connector types to include as evidence.
    pub connector_type_ids: Vec<String>,
    /// Maximum age of evidence in days.
    pub max_age_days: Option<i64>,
    /// Minimum trust rank for evidence.
    pub min_trust_rank: u8,
}

/// A test case for a criterion (docs/14 §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CriterionTest {
    /// Test name.
    pub name: String,
    /// Input predicate values (alias → JSON value).
    pub input: serde_json::Value,
    /// Expected result (true = criterion triggers).
    pub expected: bool,
    /// Expected explanation parameters.
    pub expected_explanation: Option<serde_json::Value>,
}
