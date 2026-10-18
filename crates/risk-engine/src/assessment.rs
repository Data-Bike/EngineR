//! Assessment aggregate (docs/14 §10).
//!
//! Assessment stores: target/as_of, model/version, input watermark/fingerprint,
//! factor refs, component scores, final decimal score/category, uncertainty/
//! data freshness, generated/approved actors, status draft/approved/superseded,
//! explanation.

use crate::factor::Factor;
use crate::score::{RiskLevel, ScoreResult};
use serde::{Deserialize, Serialize};

/// Assessment status (docs/14 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    /// Draft, not yet approved.
    Draft,
    /// Approved by an actor.
    Approved,
    /// Superseded by a newer assessment.
    Superseded,
}

/// Uncertainty and data freshness metrics (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Uncertainty {
    /// Data freshness: age of oldest observation in days.
    pub oldest_observation_age_days: i64,
    /// Coverage: percentage of required predicates that have values (0-100).
    pub coverage_percent: f64,
    /// Conflict count: number of conflicting sources.
    pub conflict_count: u32,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// A component score in an assessment (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentScore {
    /// Criterion ID.
    pub criterion_id: String,
    /// Score contribution.
    pub score: f64,
    /// Weight applied.
    pub weight: f64,
    /// Factor ID (if a factor was detected).
    pub factor_id: Option<String>,
}

/// A risk assessment (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assessment {
    /// Assessment ID (UUID).
    pub assessment_id: String,
    /// Target entity public ID.
    pub target_entity_id: String,
    /// As-of date (point in time for the assessment).
    pub as_of: chrono::DateTime<chrono::Utc>,
    /// Model ID.
    pub model_id: String,
    /// Model version.
    pub model_version: String,
    /// Input watermark (latest observation timestamp included).
    pub input_watermark: chrono::DateTime<chrono::Utc>,
    /// Input fingerprint (hash of all input statements/observations).
    pub input_fingerprint: String,
    /// Factor references (factor IDs).
    pub factor_ids: Vec<String>,
    /// Component scores.
    pub component_scores: Vec<ComponentScore>,
    /// Final decimal score (0-100).
    pub final_score: f64,
    /// Risk level/category.
    pub level: RiskLevel,
    /// Uncertainty metrics.
    pub uncertainty: Uncertainty,
    /// Actor who generated the assessment.
    pub generated_by: String,
    /// When the assessment was generated.
    pub generated_at: chrono::DateTime<chrono::Utc>,
    /// Actor who approved the assessment (if approved).
    pub approved_by: Option<String>,
    /// When the assessment was approved.
    pub approved_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Status.
    pub status: AssessmentStatus,
    /// Explanation (i18n key + params).
    pub explanation_key: String,
    pub explanation_params: serde_json::Value,
}

impl Assessment {
    /// Create a new draft assessment from a score result.
    pub fn new_draft(
        target_entity_id: &str,
        model_id: &str,
        model_version: &str,
        input_watermark: chrono::DateTime<chrono::Utc>,
        input_fingerprint: &str,
        factors: &[Factor],
        score_result: &ScoreResult,
        uncertainty: Uncertainty,
        generated_by: &str,
    ) -> Self {
        let factor_ids: Vec<String> = factors.iter().map(|f| f.factor_id.clone()).collect();
        let component_scores: Vec<ComponentScore> = score_result
            .components
            .iter()
            .map(|c| ComponentScore {
                criterion_id: c.criterion_id.clone(),
                score: c.score_contribution,
                weight: c.weight,
                factor_id: factors
                    .iter()
                    .find(|f| f.criterion_id == c.criterion_id)
                    .map(|f| f.factor_id.clone()),
            })
            .collect();

        Self {
            assessment_id: uuid::Uuid::now_v7().to_string(),
            target_entity_id: target_entity_id.to_string(),
            as_of: chrono::Utc::now(),
            model_id: model_id.to_string(),
            model_version: model_version.to_string(),
            input_watermark,
            input_fingerprint: input_fingerprint.to_string(),
            factor_ids,
            component_scores,
            final_score: score_result.score,
            level: score_result.level,
            uncertainty,
            generated_by: generated_by.to_string(),
            generated_at: chrono::Utc::now(),
            approved_by: None,
            approved_at: None,
            status: AssessmentStatus::Draft,
            explanation_key: "assessment.generated".to_string(),
            explanation_params: serde_json::json!({
                "score": score_result.score,
                "level": format!("{:?}", score_result.level),
                "factor_count": factors.len(),
            }),
        }
    }

    /// Approve the assessment.
    pub fn approve(&mut self, actor: &str) {
        self.status = AssessmentStatus::Approved;
        self.approved_by = Some(actor.to_string());
        self.approved_at = Some(chrono::Utc::now());
    }

    /// Supersede the assessment (mark as replaced).
    pub fn supersede(&mut self) {
        self.status = AssessmentStatus::Superseded;
    }

    /// Check if the assessment is current (not superseded).
    pub fn is_current(&self) -> bool {
        matches!(
            self.status,
            AssessmentStatus::Draft | AssessmentStatus::Approved
        )
    }
}
