//! Decision (docs/14 §10).
//!
//! Score is not the only decision. Decision is separate and can consider
//! policy/limits/human judgment (docs/14 §10).

use crate::score::RiskLevel;
use serde::{Deserialize, Serialize};

/// Kind of decision (docs/14 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionKind {
    /// Approve: risk is acceptable.
    Approve,
    /// Reject: risk is too high.
    Reject,
    /// Escalate: needs human review.
    Escalate,
    /// Defer: pending additional information.
    Defer,
    /// Conditional approve: with mitigations.
    ConditionalApprove,
}

/// Reason for a decision (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionReason {
    /// Human-readable reason (i18n key).
    pub reason_key: String,
    /// Reason parameters (for i18n substitution).
    pub params: serde_json::Value,
    /// Policy rule that triggered this decision (if any).
    pub policy_rule_id: Option<String>,
}

/// A risk decision (docs/14 §10).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decision {
    /// Decision ID (UUID).
    pub decision_id: String,
    /// Target entity public ID.
    pub target_entity_id: String,
    /// Assessment ID this decision is based on.
    pub assessment_id: String,
    /// Decision kind.
    pub kind: DecisionKind,
    /// Risk level at decision time.
    pub risk_level: RiskLevel,
    /// Score at decision time.
    pub score: f64,
    /// Reason for the decision.
    pub reason: DecisionReason,
    /// Actor who made the decision (system or user).
    pub actor: String,
    /// When the decision was made.
    pub made_at: chrono::DateTime<chrono::Utc>,
    /// Whether this decision requires approval (docs/14 §12).
    pub requires_approval: bool,
    /// Conditions (for ConditionalApprove).
    pub conditions: Vec<String>,
    /// Mitigations required.
    pub mitigations: Vec<String>,
}

impl Decision {
    /// Create a system decision based on score and level.
    pub fn system(
        target_entity_id: &str,
        assessment_id: &str,
        score: f64,
        level: RiskLevel,
        reason_key: &str,
    ) -> Self {
        let kind = match level {
            RiskLevel::Minimal | RiskLevel::Low => DecisionKind::Approve,
            RiskLevel::Medium => DecisionKind::Escalate,
            RiskLevel::High | RiskLevel::Critical => DecisionKind::Reject,
        };
        let requires_approval = matches!(level, RiskLevel::High | RiskLevel::Critical);
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            target_entity_id: target_entity_id.to_string(),
            assessment_id: assessment_id.to_string(),
            kind,
            risk_level: level,
            score,
            reason: DecisionReason {
                reason_key: reason_key.to_string(),
                params: serde_json::json!({ "score": score, "level": format!("{:?}", level) }),
                policy_rule_id: None,
            },
            actor: "system".to_string(),
            made_at: chrono::Utc::now(),
            requires_approval,
            conditions: Vec::new(),
            mitigations: Vec::new(),
        }
    }

    /// Create a manual decision by a human actor.
    pub fn manual(
        target_entity_id: &str,
        assessment_id: &str,
        kind: DecisionKind,
        score: f64,
        level: RiskLevel,
        actor: &str,
        reason_key: &str,
        conditions: Vec<String>,
        mitigations: Vec<String>,
    ) -> Self {
        Self {
            decision_id: uuid::Uuid::now_v7().to_string(),
            target_entity_id: target_entity_id.to_string(),
            assessment_id: assessment_id.to_string(),
            kind,
            risk_level: level,
            score,
            reason: DecisionReason {
                reason_key: reason_key.to_string(),
                params: serde_json::json!({ "score": score }),
                policy_rule_id: None,
            },
            actor: actor.to_string(),
            made_at: chrono::Utc::now(),
            requires_approval: false,
            conditions,
            mitigations,
        }
    }
}
