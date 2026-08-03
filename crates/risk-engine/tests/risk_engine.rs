//! Risk engine tests (docs/14 §8-12).

#![cfg(test)]

use enginer_risk_engine::{
    evaluate, Assessment, AssessmentStatus, CriterionSeverity,
    Decision, DecisionKind, EvaluationContext, Expr, Factor, FactorStatus, LogicalOp,
    ObservationContext, RiskLevel, ScoreConfig, Uncertainty, WeightedAggregator,
};

// --- AST evaluation tests ---

#[test]
fn evaluate_literal_integer() {
    let expr = Expr::literal(enginer_risk_engine::Value::Integer { value: 42 });
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Integer { value: 42 }
    );
}

#[test]
fn evaluate_literal_boolean() {
    let expr = Expr::literal(enginer_risk_engine::Value::Boolean { value: true });
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Boolean { value: true }
    );
}

#[test]
fn evaluate_compare_integer_eq() {
    use enginer_risk_engine::{ComparisonOp, Value};
    let expr = Expr::compare(
        ComparisonOp::Eq,
        Expr::literal(Value::Integer { value: 5 }),
        Expr::literal(Value::Integer { value: 5 }),
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_compare_integer_lt() {
    use enginer_risk_engine::{ComparisonOp, Value};
    let expr = Expr::compare(
        ComparisonOp::Lt,
        Expr::literal(Value::Integer { value: 3 }),
        Expr::literal(Value::Integer { value: 5 }),
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_compare_string_eq() {
    use enginer_risk_engine::{ComparisonOp, Value};
    let expr = Expr::compare(
        ComparisonOp::Eq,
        Expr::literal(Value::Text {
            value: "hello".to_string(),
        }),
        Expr::literal(Value::Text {
            value: "hello".to_string(),
        }),
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_compare_string_normalized_eq() {
    use enginer_risk_engine::{ComparisonOp, Value};
    let expr = Expr::compare(
        ComparisonOp::NormalizedEq,
        Expr::literal(Value::Text {
            value: "  Hello  ".to_string(),
        }),
        Expr::literal(Value::Text {
            value: "hello".to_string(),
        }),
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_logical_all_true() {
    use enginer_risk_engine::Value;
    let expr = Expr::logical(
        LogicalOp::All,
        vec![
            Expr::literal(Value::Boolean { value: true }),
            Expr::literal(Value::Boolean { value: true }),
        ],
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_logical_all_false() {
    use enginer_risk_engine::Value;
    let expr = Expr::logical(
        LogicalOp::All,
        vec![
            Expr::literal(Value::Boolean { value: true }),
            Expr::literal(Value::Boolean { value: false }),
        ],
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: false });
}

#[test]
fn evaluate_logical_any_true() {
    use enginer_risk_engine::Value;
    let expr = Expr::logical(
        LogicalOp::Any,
        vec![
            Expr::literal(Value::Boolean { value: false }),
            Expr::literal(Value::Boolean { value: true }),
        ],
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_unary_not() {
    use enginer_risk_engine::{UnaryOp, Value};
    let expr = Expr::unary(UnaryOp::Not, Expr::literal(Value::Boolean { value: true }));
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: false });
}

#[test]
fn evaluate_existence_true() {
    let expr = Expr::exists("person.inn");
    let mut ctx = EvaluationContext::default();
    ctx.predicate_values
        .insert("person.inn".to_string(), vec![enginer_risk_engine::Value::Text {
            value: "1234567890".to_string(),
        }]);
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Boolean { value: true }
    );
}

#[test]
fn evaluate_existence_false() {
    let expr = Expr::exists("person.inn");
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Boolean { value: false }
    );
}

#[test]
fn evaluate_range_inclusive() {
    use enginer_risk_engine::Value;
    let expr = Expr::range(
        Expr::literal(Value::Integer { value: 5 }),
        Expr::literal(Value::Integer { value: 1 }),
        Expr::literal(Value::Integer { value: 10 }),
        true,
        true,
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: true });
}

#[test]
fn evaluate_range_outside() {
    use enginer_risk_engine::Value;
    let expr = Expr::range(
        Expr::literal(Value::Integer { value: 15 }),
        Expr::literal(Value::Integer { value: 1 }),
        Expr::literal(Value::Integer { value: 10 }),
        true,
        true,
    );
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Boolean { value: false });
}

#[test]
fn evaluate_arithmetic_add() {
    use enginer_risk_engine::{BinaryOp, Value};
    let expr = Expr {
        kind: enginer_risk_engine::ExprKind::Arithmetic {
            op: BinaryOp::Add,
            left: Box::new(Expr::literal(Value::Integer { value: 3 })),
            right: Box::new(Expr::literal(Value::Integer { value: 4 })),
        },
    };
    let ctx = EvaluationContext::default();
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(result, Value::Integer { value: 7 });
}

#[test]
fn evaluate_source_freshness() {
    let expr = Expr {
        kind: enginer_risk_engine::ExprKind::SourceFreshness {
            source_connector_type_id: "fns-registry".to_string(),
            max_age_days: 30,
        },
    };
    let ctx = EvaluationContext {
        observations: vec![ObservationContext {
            connector_type_id: "fns-registry".to_string(),
            age_days: 10,
            trust_rank: 75,
        }],
        ..Default::default()
    };
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Boolean { value: true }
    );
}

#[test]
fn evaluate_source_freshness_stale() {
    let expr = Expr {
        kind: enginer_risk_engine::ExprKind::SourceFreshness {
            source_connector_type_id: "fns-registry".to_string(),
            max_age_days: 30,
        },
    };
    let ctx = EvaluationContext {
        observations: vec![ObservationContext {
            connector_type_id: "fns-registry".to_string(),
            age_days: 60,
            trust_rank: 75,
        }],
        ..Default::default()
    };
    let result = evaluate(&expr, &ctx).unwrap();
    assert_eq!(
        result,
        enginer_risk_engine::Value::Boolean { value: false }
    );
}

// --- Factor tests ---

#[test]
fn factor_new_is_active() {
    let factor = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::High,
        "50.0",
        vec![],
        serde_json::json!({}),
    );
    assert_eq!(factor.status, FactorStatus::Active);
    assert!(factor.is_active());
    assert!(!factor.dedup_key.is_empty());
}

#[test]
fn factor_resolve() {
    let mut factor = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::High,
        "50.0",
        vec![],
        serde_json::json!({}),
    );
    factor.resolve();
    assert_eq!(factor.status, FactorStatus::Resolved);
    assert!(!factor.is_active());
    assert!(factor.resolved_at.is_some());
}

// --- Score tests ---

#[test]
fn risk_level_from_score() {
    assert_eq!(RiskLevel::from_score(0.0), RiskLevel::Minimal);
    assert_eq!(RiskLevel::from_score(19.9), RiskLevel::Minimal);
    assert_eq!(RiskLevel::from_score(20.0), RiskLevel::Low);
    assert_eq!(RiskLevel::from_score(39.9), RiskLevel::Low);
    assert_eq!(RiskLevel::from_score(40.0), RiskLevel::Medium);
    assert_eq!(RiskLevel::from_score(59.9), RiskLevel::Medium);
    assert_eq!(RiskLevel::from_score(60.0), RiskLevel::High);
    assert_eq!(RiskLevel::from_score(79.9), RiskLevel::High);
    assert_eq!(RiskLevel::from_score(80.0), RiskLevel::Critical);
    assert_eq!(RiskLevel::from_score(100.0), RiskLevel::Critical);
}

#[test]
fn score_aggregator_no_factors() {
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let result = agg.aggregate(&[]);
    assert_eq!(result.score, 0.0);
    assert_eq!(result.level, RiskLevel::Minimal);
}

#[test]
fn score_aggregator_single_factor() {
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let factor = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::High,
        "50.0",
        vec![],
        serde_json::json!({}),
    );
    let result = agg.aggregate(&[factor]);
    assert!(result.score > 0.0);
    assert_eq!(result.components.len(), 1);
}

#[test]
fn score_aggregator_multiple_factors() {
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let f1 = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::Low,
        "10.0",
        vec![],
        serde_json::json!({}),
    );
    let f2 = Factor::new(
        "criterion-002",
        "1.0.0",
        "ent-001",
        CriterionSeverity::High,
        "50.0",
        vec![],
        serde_json::json!({}),
    );
    let result = agg.aggregate(&[f1, f2]);
    assert!(result.score > 0.0);
    assert_eq!(result.components.len(), 2);
    assert!(result.penalty > 0.0);
}

// --- Decision tests ---

#[test]
fn decision_system_approve_low_risk() {
    let decision = Decision::system(
        "ent-001",
        "assessment-001",
        10.0,
        RiskLevel::Minimal,
        "risk.low",
    );
    assert_eq!(decision.kind, DecisionKind::Approve);
    assert!(!decision.requires_approval);
    assert_eq!(decision.actor, "system");
}

#[test]
fn decision_system_reject_critical_risk() {
    let decision = Decision::system(
        "ent-001",
        "assessment-001",
        90.0,
        RiskLevel::Critical,
        "risk.critical",
    );
    assert_eq!(decision.kind, DecisionKind::Reject);
    assert!(decision.requires_approval);
}

#[test]
fn decision_system_escalate_medium_risk() {
    let decision = Decision::system(
        "ent-001",
        "assessment-001",
        50.0,
        RiskLevel::Medium,
        "risk.medium",
    );
    assert_eq!(decision.kind, DecisionKind::Escalate);
}

#[test]
fn decision_manual_with_conditions() {
    let decision = Decision::manual(
        "ent-001",
        "assessment-001",
        DecisionKind::ConditionalApprove,
        45.0,
        RiskLevel::Medium,
        "user-001",
        "manual.review",
        vec!["additional_document".to_string()],
        vec!["monitor_monthly".to_string()],
    );
    assert_eq!(decision.kind, DecisionKind::ConditionalApprove);
    assert_eq!(decision.actor, "user-001");
    assert_eq!(decision.conditions.len(), 1);
    assert_eq!(decision.mitigations.len(), 1);
}

// --- Assessment tests ---

#[test]
fn assessment_new_draft() {
    let factor = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::Medium,
        "25.0",
        vec![],
        serde_json::json!({}),
    );
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let score_result = agg.aggregate(&[factor.clone()]);
    let uncertainty = Uncertainty {
        oldest_observation_age_days: 5,
        coverage_percent: 95.0,
        conflict_count: 0,
        confidence: 0.9,
    };
    let assessment = Assessment::new_draft(
        "ent-001",
        "default",
        "1.0.0",
        chrono::Utc::now(),
        "fingerprint-001",
        &[factor],
        &score_result,
        uncertainty,
        "system",
    );
    assert_eq!(assessment.status, AssessmentStatus::Draft);
    assert!(assessment.is_current());
    assert_eq!(assessment.generated_by, "system");
    assert!(assessment.approved_by.is_none());
}

#[test]
fn assessment_approve() {
    let factor = Factor::new(
        "criterion-001",
        "1.0.0",
        "ent-001",
        CriterionSeverity::Low,
        "10.0",
        vec![],
        serde_json::json!({}),
    );
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let score_result = agg.aggregate(&[factor]);
    let uncertainty = Uncertainty {
        oldest_observation_age_days: 1,
        coverage_percent: 100.0,
        conflict_count: 0,
        confidence: 1.0,
    };
    let mut assessment = Assessment::new_draft(
        "ent-001",
        "default",
        "1.0.0",
        chrono::Utc::now(),
        "fp-001",
        &[],
        &score_result,
        uncertainty,
        "system",
    );
    assessment.approve("user-001");
    assert_eq!(assessment.status, AssessmentStatus::Approved);
    assert_eq!(assessment.approved_by.as_deref(), Some("user-001"));
    assert!(assessment.approved_at.is_some());
}

#[test]
fn assessment_supersede() {
    let agg = WeightedAggregator::new(ScoreConfig::default());
    let score_result = agg.aggregate(&[]);
    let uncertainty = Uncertainty {
        oldest_observation_age_days: 0,
        coverage_percent: 100.0,
        conflict_count: 0,
        confidence: 1.0,
    };
    let mut assessment = Assessment::new_draft(
        "ent-001",
        "default",
        "1.0.0",
        chrono::Utc::now(),
        "fp-001",
        &[],
        &score_result,
        uncertainty,
        "system",
    );
    assessment.supersede();
    assert_eq!(assessment.status, AssessmentStatus::Superseded);
    assert!(!assessment.is_current());
}

// --- Criterion severity tests ---

#[test]
fn criterion_severity_weight() {
    assert_eq!(CriterionSeverity::Info.weight_contribution(), 0);
    assert_eq!(CriterionSeverity::Low.weight_contribution(), 10);
    assert_eq!(CriterionSeverity::Medium.weight_contribution(), 25);
    assert_eq!(CriterionSeverity::High.weight_contribution(), 50);
    assert_eq!(CriterionSeverity::Critical.weight_contribution(), 100);
}
