//! Integration tests for the check flow orchestrator (docs/13 §8, docs/14 §4-12).
//!
//! These tests use in-memory mock ports to verify the end-to-end flow:
//! check request → connector fetch → mapping → criteria evaluation →
//! factor detection → score → assessment → decision → workflow → tasks.

#![cfg(test)]

use enginer_connectors::{
    IdentityResolution, MappedPredicate, MappingResult, ObservationStatus, ProvenanceMeta,
    RawPayloadPolicy, SourceObservation, UnmappedField,
};
use enginer_integration::{
    AssessmentPort, AuditPort, CheckFlowInput, CheckFlowOrchestrator, CheckFlowState,
    ConnectorPort, CriterionPort, DecisionPort, FactorPort, TaskPort, WorkflowPort,
};
use enginer_risk_engine::{
    Assessment, Criterion, CriterionSeverity, CriterionStatus, Decision, EvidenceSelector, Expr,
    ExprKind, Factor, Value,
};
use enginer_security::AuditEntry;
use enginer_workflow::Task;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// --- Mock ports ---

struct MockCriterionPort {
    criteria: Vec<Criterion>,
}

impl CriterionPort for MockCriterionPort {
    fn load_criteria(
        &self,
        _tenant_id: &str,
        _target_type: &str,
    ) -> Result<Vec<Criterion>, String> {
        Ok(self.criteria.clone())
    }
}

struct MockConnectorPort {
    observations: Vec<SourceObservation>,
    mapping_result: MappingResult,
}

impl ConnectorPort for MockConnectorPort {
    fn fetch_observations(
        &self,
        _connector_instance_id: &str,
        _target_entity_id: &str,
    ) -> Result<Vec<SourceObservation>, String> {
        Ok(self.observations.clone())
    }

    fn map_observations(
        &self,
        _observations: &[SourceObservation],
    ) -> Result<MappingResult, String> {
        Ok(self.mapping_result.clone())
    }
}

struct MockFactorPort {
    stored: Mutex<Vec<Factor>>,
}

impl FactorPort for MockFactorPort {
    fn store_factor(&self, factor: &Factor) -> Result<(), String> {
        self.stored.lock().unwrap().push(factor.clone());
        Ok(())
    }

    fn load_factors(&self, _target_entity_id: &str) -> Result<Vec<Factor>, String> {
        Ok(self.stored.lock().unwrap().clone())
    }

    fn resolve_factor(&self, _factor_id: &str) -> Result<(), String> {
        Ok(())
    }
}

struct MockAssessmentPort {
    stored: Mutex<Vec<Assessment>>,
}

impl AssessmentPort for MockAssessmentPort {
    fn store_assessment(&self, assessment: &Assessment) -> Result<(), String> {
        self.stored.lock().unwrap().push(assessment.clone());
        Ok(())
    }

    fn load_latest_assessment(
        &self,
        _target_entity_id: &str,
    ) -> Result<Option<Assessment>, String> {
        Ok(self.stored.lock().unwrap().last().cloned())
    }
}

struct MockDecisionPort {
    stored: Mutex<Vec<Decision>>,
}

impl DecisionPort for MockDecisionPort {
    fn store_decision(&self, decision: &Decision) -> Result<(), String> {
        self.stored.lock().unwrap().push(decision.clone());
        Ok(())
    }

    fn load_latest_decision(&self, _target_entity_id: &str) -> Result<Option<Decision>, String> {
        Ok(self.stored.lock().unwrap().last().cloned())
    }
}

struct MockWorkflowPort {
    instances: Mutex<Vec<String>>,
}

impl WorkflowPort for MockWorkflowPort {
    fn start_workflow(
        &self,
        definition_id: &str,
        _target_entity_id: &str,
        _variables: HashMap<String, serde_json::Value>,
    ) -> Result<String, String> {
        let instance_id = uuid::Uuid::now_v7().to_string();
        let _ = definition_id;
        self.instances.lock().unwrap().push(instance_id.clone());
        Ok(instance_id)
    }

    fn transition_workflow(
        &self,
        _instance_id: &str,
        _to: &str,
        _trigger: &str,
        _actor: &str,
    ) -> Result<(), String> {
        Ok(())
    }

    fn get_workflow(&self, instance_id: &str) -> Result<Option<String>, String> {
        Ok(self
            .instances
            .lock()
            .unwrap()
            .iter()
            .find(|id| *id == instance_id)
            .cloned())
    }
}

struct MockTaskPort {
    tasks: Mutex<Vec<Task>>,
}

impl TaskPort for MockTaskPort {
    fn create_task(&self, task: &Task) -> Result<(), String> {
        self.tasks.lock().unwrap().push(task.clone());
        Ok(())
    }

    fn load_tasks(&self, _workflow_instance_id: &str) -> Result<Vec<Task>, String> {
        Ok(self.tasks.lock().unwrap().clone())
    }

    fn complete_task(&self, _task_id: &str) -> Result<(), String> {
        Ok(())
    }
}

struct MockAuditPort {
    entries: Mutex<Vec<AuditEntry>>,
}

impl MockAuditPort {
    fn new() -> Self {
        Self {
            entries: Mutex::new(Vec::new()),
        }
    }

    fn count(&self) -> usize {
        self.entries.lock().unwrap().len()
    }

    fn actions(&self) -> Vec<String> {
        self.entries
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.action.clone())
            .collect()
    }
}

impl AuditPort for MockAuditPort {
    fn record(&self, entry: &AuditEntry) -> Result<(), String> {
        self.entries.lock().unwrap().push(entry.clone());
        Ok(())
    }
}

// --- Helpers ---

fn make_test_observations() -> Vec<SourceObservation> {
    vec![SourceObservation {
        observation_id: "obs-001".to_string(),
        status: ObservationStatus::Fresh,
        provenance: ProvenanceMeta {
            source_connector_type_id: "fns_egru".to_string(),
            source_record_id: "rec-001".to_string(),
            retrieved_at: chrono::Utc::now(),
            source_url: None,
            content_hash: "hash-001".to_string(),
            trust_rank: 75,
            purpose: "kyc".to_string(),
        },
        data: serde_json::json!({"inn": "1234567890", "ogrn": "1234567890123"}),
        adverse: false,
        confidence: "0.95".to_string(),
        raw_payload_policy: RawPayloadPolicy::HashOnly,
        raw_payload_hash: Some("hash-001".to_string()),
        diagnostics: None,
    }]
}

fn make_test_mapping_result() -> MappingResult {
    MappingResult {
        mapped_predicates: vec![
            MappedPredicate {
                predicate_id: "pred-inn".to_string(),
                predicate_alias: "inn".to_string(),
                canonical_value: "1234567890".to_string(),
                value_kind: "text".to_string(),
                observation_id: "obs-001".to_string(),
                source_field: "inn".to_string(),
            },
            MappedPredicate {
                predicate_id: "pred-ogrn".to_string(),
                predicate_alias: "ogrn".to_string(),
                canonical_value: "1234567890123".to_string(),
                value_kind: "text".to_string(),
                observation_id: "obs-001".to_string(),
                source_field: "ogrn".to_string(),
            },
        ],
        unmapped: Vec::new(),
        identity_resolution: IdentityResolution {
            outcome: "no_match".to_string(),
            entity_public_id: None,
            candidates: Vec::new(),
            score: None,
        },
    }
}

fn make_test_criteria(triggered: bool) -> Vec<Criterion> {
    let expr = if triggered {
        Expr {
            kind: ExprKind::Literal {
                value: Value::Boolean { value: true },
            },
        }
    } else {
        Expr {
            kind: ExprKind::Literal {
                value: Value::Boolean { value: false },
            },
        }
    };

    vec![Criterion {
        criterion_id: "crit-001".to_string(),
        version: "1.0.0".to_string(),
        name: "High risk criterion".to_string(),
        expression: expr,
        severity: CriterionSeverity::High,
        weight: "1.0".to_string(),
        explanation_key: "criterion.high_risk".to_string(),
        evidence_selector: EvidenceSelector {
            connector_type_ids: Vec::new(),
            max_age_days: None,
            min_trust_rank: 0,
        },
        valid_from: chrono::Utc::now() - chrono::Duration::days(1),
        valid_to: None,
        owner: "risk-team".to_string(),
        status: CriterionStatus::Active,
        tests: Vec::new(),
    }]
}

fn make_orchestrator(
    criteria: Vec<Criterion>,
    observations: Vec<SourceObservation>,
    mapping_result: MappingResult,
) -> CheckFlowOrchestrator {
    CheckFlowOrchestrator::new(
        Arc::new(MockCriterionPort { criteria }),
        Arc::new(MockConnectorPort {
            observations,
            mapping_result,
        }),
        Arc::new(MockFactorPort {
            stored: Mutex::new(Vec::new()),
        }),
        Arc::new(MockAssessmentPort {
            stored: Mutex::new(Vec::new()),
        }),
        Arc::new(MockDecisionPort {
            stored: Mutex::new(Vec::new()),
        }),
        Arc::new(MockWorkflowPort {
            instances: Mutex::new(Vec::new()),
        }),
        Arc::new(MockTaskPort {
            tasks: Mutex::new(Vec::new()),
        }),
    )
}

fn make_input() -> CheckFlowInput {
    CheckFlowInput {
        tenant_id: "tenant-A".to_string(),
        target_entity_id: "ent-001".to_string(),
        target_type: "legal_entity".to_string(),
        check_kind: "kyc".to_string(),
        connector_instance_id: "conn-001".to_string(),
        actor: "system".to_string(),
        force: true,
    }
}

// --- Tests ---

#[test]
fn check_flow_completes_with_no_factors() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false), // no criteria triggered
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    assert_eq!(result.state, CheckFlowState::Completed);
    assert_eq!(result.observation_count, 1);
    assert_eq!(result.criteria_evaluated, 1);
    assert_eq!(result.factors_detected, 0);
    assert_eq!(result.final_score, 0.0);
    assert!(result.assessment_id.is_some());
    assert!(result.decision.is_some());
    // No escalation → no workflow
    assert!(result.workflow_instance_id.is_none());
    assert_eq!(result.tasks_created, 0);
}

#[test]
fn check_flow_detects_factors_and_scores() {
    let orchestrator = make_orchestrator(
        make_test_criteria(true), // criteria triggered
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    assert_eq!(result.state, CheckFlowState::Completed);
    assert_eq!(result.factors_detected, 1);
    // High severity = 50 score contribution
    assert!(result.final_score > 0.0);
    assert!(result.assessment_id.is_some());
    assert!(result.decision.is_some());
}

#[test]
fn check_flow_creates_workflow_on_escalation() {
    // High severity criterion → high score → escalate
    let orchestrator = make_orchestrator(
        make_test_criteria(true),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    // If decision is Escalate or Reject, workflow should be started
    if let Some(decision) = &result.decision {
        if matches!(
            decision.kind,
            enginer_risk_engine::DecisionKind::Escalate | enginer_risk_engine::DecisionKind::Reject
        ) {
            assert!(
                result.workflow_instance_id.is_some(),
                "workflow should be started on escalation"
            );
            assert!(
                result.tasks_created > 0,
                "tasks should be created on escalation"
            );
        }
    }
}

#[test]
fn check_flow_state_transitions() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    // Final state should be Completed
    assert_eq!(result.state, CheckFlowState::Completed);
    assert!(result.state.is_terminal());
}

#[test]
fn check_flow_failed_state_is_terminal() {
    assert!(CheckFlowState::Failed.is_terminal());
    assert!(CheckFlowState::Completed.is_terminal());
    assert!(!CheckFlowState::Initiated.is_terminal());
    assert!(!CheckFlowState::Fetching.is_terminal());
}

#[test]
fn check_flow_handles_empty_observations() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        Vec::new(), // no observations
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    assert_eq!(result.state, CheckFlowState::Completed);
    assert_eq!(result.observation_count, 0);
    // Should have a warning about no observations
    assert!(result
        .warnings
        .iter()
        .any(|w| w.contains("no observations")));
}

#[test]
fn check_flow_handles_no_criteria() {
    let orchestrator = make_orchestrator(
        Vec::new(), // no criteria
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    assert_eq!(result.state, CheckFlowState::Completed);
    assert_eq!(result.criteria_evaluated, 0);
    assert_eq!(result.factors_detected, 0);
}

#[test]
fn check_flow_result_has_correct_target() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let input = make_input();
    let expected_target = input.target_entity_id.clone();
    let result = orchestrator.execute(input).unwrap();

    assert_eq!(result.target_entity_id, expected_target);
}

#[test]
fn check_flow_decision_has_target_entity() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    let decision = result.decision.expect("decision should exist");
    assert_eq!(decision.target_entity_id, "ent-001");
}

#[test]
fn check_flow_completed_at_is_set() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let before = chrono::Utc::now();
    let result = orchestrator.execute(make_input()).unwrap();
    let after = chrono::Utc::now();

    assert!(result.completed_at >= before);
    assert!(result.completed_at <= after);
}

#[test]
fn check_flow_check_id_is_uuid() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    let result = orchestrator.execute(make_input()).unwrap();

    // Should be a valid UUID
    assert!(uuid::Uuid::parse_str(&result.check_id).is_ok());
}

#[test]
fn check_flow_records_audit_events() {
    let audit = Arc::new(MockAuditPort::new());
    let orchestrator = make_orchestrator(
        make_test_criteria(true),
        make_test_observations(),
        make_test_mapping_result(),
    )
    .with_audit_port(audit.clone());

    let result = orchestrator.execute(make_input()).unwrap();
    assert_eq!(result.state, CheckFlowState::Completed);

    // Should have 3 audit entries: initiated, decision, completed
    let actions = audit.actions();
    assert_eq!(
        actions.len(),
        3,
        "expected 3 audit events, got {:?}",
        actions
    );
    assert!(actions.contains(&"check.run".to_string()));
    assert!(actions.contains(&"check.decision".to_string()));
    assert!(actions.contains(&"check.completed".to_string()));
}

#[test]
fn check_flow_audit_decision_recorded_with_effect() {
    let audit = Arc::new(MockAuditPort::new());
    let orchestrator = make_orchestrator(
        make_test_criteria(true), // triggered → high score → escalate
        make_test_observations(),
        make_test_mapping_result(),
    )
    .with_audit_port(audit.clone());

    orchestrator.execute(make_input()).unwrap();

    let entries = audit.entries.lock().unwrap();
    let decision_entry = entries
        .iter()
        .find(|e| e.action == "check.decision")
        .expect("decision audit entry should exist");
    assert!(
        decision_entry.decision.is_some(),
        "decision entry should have effect"
    );
}

#[test]
fn check_flow_without_audit_port_still_works() {
    let orchestrator = make_orchestrator(
        make_test_criteria(false),
        make_test_observations(),
        make_test_mapping_result(),
    );
    // No audit port wired — should not fail
    let result = orchestrator.execute(make_input()).unwrap();
    assert_eq!(result.state, CheckFlowState::Completed);
}
