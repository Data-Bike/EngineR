//! Score aggregation and risk level mapping (docs/14 §10).
//!
//! Score is not the only decision. Decision is separate and can consider
//! policy/limits/human judgment (docs/14 §10).

use crate::factor::Factor;
use serde::{Deserialize, Serialize};

/// Risk level (docs/14 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    /// Minimal risk (score 0-20).
    Minimal,
    /// Low risk (score 20-40).
    Low,
    /// Medium risk (score 40-60).
    Medium,
    /// High risk (score 60-80).
    High,
    /// Critical risk (score 80-100).
    Critical,
}

impl RiskLevel {
    /// Map a numeric score (0-100) to a risk level.
    pub fn from_score(score: f64) -> Self {
        if score < 20.0 {
            Self::Minimal
        } else if score < 40.0 {
            Self::Low
        } else if score < 60.0 {
            Self::Medium
        } else if score < 80.0 {
            Self::High
        } else {
            Self::Critical
        }
    }

    /// Get the numeric rank (0-4).
    pub fn rank(self) -> u8 {
        match self {
            Self::Minimal => 0,
            Self::Low => 1,
            Self::Medium => 2,
            Self::High => 3,
            Self::Critical => 4,
        }
    }
}

/// Score configuration (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreConfig {
    /// Model ID.
    pub model_id: String,
    /// Model version.
    pub model_version: String,
    /// Whether to use weighted average (true) or max (false).
    pub weighted_average: bool,
    /// Maximum possible score.
    pub max_score: f64,
    /// Penalty per unresolved factor (added to base score).
    pub per_factor_penalty: f64,
    /// Cap on total penalty.
    pub max_penalty: f64,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            model_id: "default".to_string(),
            model_version: "1.0.0".to_string(),
            weighted_average: true,
            max_score: 100.0,
            per_factor_penalty: 5.0,
            max_penalty: 50.0,
        }
    }
}

/// Score result (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoreResult {
    /// Final decimal score (0 - max_score).
    pub score: f64,
    /// Risk level derived from score.
    pub level: RiskLevel,
    /// Component scores (criterion ID → score contribution).
    pub components: Vec<ComponentScoreEntry>,
    /// Total penalty applied.
    pub penalty: f64,
}

/// A component score entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentScoreEntry {
    pub criterion_id: String,
    pub score_contribution: f64,
    pub weight: f64,
}

/// Weighted score aggregator (docs/14 §10).
pub struct WeightedAggregator {
    config: ScoreConfig,
}

impl WeightedAggregator {
    /// Create a new aggregator with the given config.
    pub fn new(config: ScoreConfig) -> Self {
        Self { config }
    }

    /// Aggregate factors into a final score.
    pub fn aggregate(&self, factors: &[Factor]) -> ScoreResult {
        let active_factors: Vec<&Factor> = factors.iter().filter(|f| f.is_active()).collect();

        if active_factors.is_empty() {
            return ScoreResult {
                score: 0.0,
                level: RiskLevel::Minimal,
                components: Vec::new(),
                penalty: 0.0,
            };
        }

        let mut components = Vec::new();
        let mut total_weighted = 0.0;
        let mut total_weight = 0.0;

        for factor in &active_factors {
            let contribution: f64 = factor.score_contribution.parse().unwrap_or(0.0);
            let weight: f64 = factor.severity.weight_contribution() as f64;
            total_weighted += contribution * weight;
            total_weight += weight;
            components.push(ComponentScoreEntry {
                criterion_id: factor.criterion_id.clone(),
                score_contribution: contribution,
                weight,
            });
        }

        let base_score = if self.config.weighted_average && total_weight > 0.0 {
            total_weighted / total_weight
        } else {
            // Max mode: take the highest contribution
            active_factors
                .iter()
                .map(|f| f.score_contribution.parse::<f64>().unwrap_or(0.0))
                .fold(0.0_f64, f64::max)
        };

        let penalty = (active_factors.len() as f64 * self.config.per_factor_penalty)
            .min(self.config.max_penalty);

        let final_score = (base_score + penalty).min(self.config.max_score);
        let level = RiskLevel::from_score(final_score);

        ScoreResult {
            score: final_score,
            level,
            components,
            penalty,
        }
    }
}
