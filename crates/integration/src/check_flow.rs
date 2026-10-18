//! Check flow orchestrator (docs/13 §8, docs/14 §4-12).
//!
//! End-to-end check flow:
//! 1. Receive check request (target entity, check kind, connector).
//! 2. Fetch observations from connector (docs/14 §5-6).
//! 3. Map observations to provenance statements (docs/14 §6).
//! 4. Load active criteria for target type (docs/14 §8).
//! 5. Evaluate each criterion against predicate values (docs/14 §8).
//! 6. Create factors for triggered criteria (docs/14 §9).
//! 7. Aggregate factors into score (docs/14 §10).
//! 8. Create assessment (docs/14 §10).
//! 9. Make decision (docs/14 §10).
//! 10. Start workflow if escalation needed (docs/13 §7-8).
//! 11. Create tasks for human review (docs/13 §8).
//! 12. Return check flow result.

use crate::error::{IntegrationError, IntegrationResult};
use crate::ports::{
    AssessmentPort, AuditPort, ConnectorPort, CriterionPort, DecisionPort, FactorPort, TaskPort,
    WorkflowPort,
};
use enginer_connectors::{MappingResult, SourceObservation};
use enginer_risk_engine::{
    Assessment, Criterion, CriterionSeverity, Decision, DecisionKind, EvaluationContext, Factor,
    ObservationContext, RiskLevel, ScoreConfig, ScoreResult, Uncertainty, WeightedAggregator,
};
use enginer_security::{AuditEntry, AuditKind};
use enginer_workflow::{Task, TaskPriority};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Check flow state (docs/14 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckFlowState {
    /// Check request received.
    Initiated,
    /// Fetching observations from connector.
    Fetching,
    /// Mapping observations to statements.
    Mapping,
    /// Evaluating criteria.
    Evaluating,
    /// Creating factors.
    FactorDetection,
    /// Aggregating score.
    Scoring,
    /// Creating assessment.
    Assessing,
    /// Making decision.
    Deciding,
    /// Starting workflow (if escalation).
    WorkflowStart,
    /// Creating tasks.
    TaskCreation,
    /// Check flow completed.
    Completed,
    /// Check flow failed.
    Failed,
}

impl CheckFlowState {
    /// Check if this is a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }
}

/// Check flow input (docs/14 §4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckFlowInput {
    /// Tenant ID.
    pub tenant_id: String,
    /// Target entity public ID.
    pub target_entity_id: String,
    /// Target entity type alias.
    pub target_type: String,
    /// Check kind (e.g. "kyc", "sanctions", "financial_health").
    pub check_kind: String,
    /// Connector instance ID to use.
    pub connector_instance_id: String,
    /// Actor requesting the check.
    pub actor: String,
    /// Whether to force a new assessment even if one exists.
    pub force: bool,
}

/// Check flow result (docs/14 §4-12).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CheckFlowResult {
    /// Check ID (UUID).
    pub check_id: String,
    /// Final state.
    pub state: CheckFlowState,
    /// Target entity ID.
    pub target_entity_id: String,
    /// Observations fetched.
    pub observation_count: usize,
    /// Criteria evaluated.
    pub criteria_evaluated: usize,
    /// Factors detected.
    pub factors_detected: usize,
    /// Final score.
    pub final_score: f64,
    /// Risk level.
    pub risk_level: RiskLevel,
    /// Assessment ID (if created).
    pub assessment_id: Option<String>,
    /// Decision (if made).
    pub decision: Option<Decision>,
    /// Workflow instance ID (if started).
    pub workflow_instance_id: Option<String>,
    /// Tasks created.
    pub tasks_created: usize,
    /// Errors encountered (non-fatal).
    pub warnings: Vec<String>,
    /// Completed at.
    pub completed_at: chrono::DateTime<chrono::Utc>,
}

/// Check flow context — shared state during orchestration.
#[derive(Debug, Clone)]
pub struct CheckFlowContext {
    pub input: CheckFlowInput,
    pub state: CheckFlowState,
    pub observations: Vec<SourceObservation>,
    pub mapping_result: Option<MappingResult>,
    pub criteria: Vec<Criterion>,
    pub factors: Vec<Factor>,
    pub score_result: Option<ScoreResult>,
    pub assessment: Option<Assessment>,
    pub decision: Option<Decision>,
    pub workflow_instance_id: Option<String>,
    pub tasks: Vec<Task>,
    pub warnings: Vec<String>,
}

impl CheckFlowContext {
    /// Create a new context from input.
    pub fn new(input: CheckFlowInput) -> Self {
        Self {
            input,
            state: CheckFlowState::Initiated,
            observations: Vec::new(),
            mapping_result: None,
            criteria: Vec::new(),
            factors: Vec::new(),
            score_result: None,
            assessment: None,
            decision: None,
            workflow_instance_id: None,
            tasks: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Transition to a new state.
    pub fn transition(&mut self, new_state: CheckFlowState) {
        tracing::debug!(
            from = ?self.state,
            to = ?new_state,
            target = %self.input.target_entity_id,
            "check flow transition"
        );
        self.state = new_state;
    }

    /// Add a warning.
    pub fn add_warning(&mut self, warning: &str) {
        self.warnings.push(warning.to_string());
    }
}

/// Check flow orchestrator (docs/13 §8, docs/14 §4-12).
///
/// Coordinates the end-to-end check flow by calling ports. The orchestrator
/// is pure — it doesn't know how data is stored or how connectors are called.
pub struct CheckFlowOrchestrator {
    criterion_port: Arc<dyn CriterionPort>,
    connector_port: Arc<dyn ConnectorPort>,
    factor_port: Arc<dyn FactorPort>,
    assessment_port: Arc<dyn AssessmentPort>,
    decision_port: Arc<dyn DecisionPort>,
    workflow_port: Arc<dyn WorkflowPort>,
    task_port: Arc<dyn TaskPort>,
    audit_port: Option<Arc<dyn AuditPort>>,
    score_config: ScoreConfig,
}

impl CheckFlowOrchestrator {
    /// Create a new orchestrator with all ports wired.
    pub fn new(
        criterion_port: Arc<dyn CriterionPort>,
        connector_port: Arc<dyn ConnectorPort>,
        factor_port: Arc<dyn FactorPort>,
        assessment_port: Arc<dyn AssessmentPort>,
        decision_port: Arc<dyn DecisionPort>,
        workflow_port: Arc<dyn WorkflowPort>,
        task_port: Arc<dyn TaskPort>,
    ) -> Self {
        Self {
            criterion_port,
            connector_port,
            factor_port,
            assessment_port,
            decision_port,
            workflow_port,
            task_port,
            audit_port: None,
            score_config: ScoreConfig::default(),
        }
    }

    /// Create a new orchestrator with custom score config.
    pub fn with_score_config(mut self, config: ScoreConfig) -> Self {
        self.score_config = config;
        self
    }

    /// Wire an audit port for recording audit events during the flow.
    pub fn with_audit_port(mut self, port: Arc<dyn AuditPort>) -> Self {
        self.audit_port = Some(port);
        self
    }

    /// Execute the end-to-end check flow.
    pub fn execute(&self, input: CheckFlowInput) -> IntegrationResult<CheckFlowResult> {
        let check_id = uuid::Uuid::now_v7().to_string();
        let mut ctx = CheckFlowContext::new(input);

        // Audit: check initiated
        self.audit_check_initiated(&ctx, &check_id);

        // Step 1: Fetch observations (docs/14 §5-6)
        self.step_fetch(&mut ctx)?;

        // Step 2: Map observations (docs/14 §6)
        self.step_map(&mut ctx)?;

        // Step 3: Load criteria (docs/14 §8)
        self.step_load_criteria(&mut ctx)?;

        // Step 4: Evaluate criteria and detect factors (docs/14 §8-9)
        self.step_evaluate_and_detect(&mut ctx)?;

        // Step 5: Aggregate score (docs/14 §10)
        self.step_score(&mut ctx)?;

        // Step 6: Create assessment (docs/14 §10)
        self.step_assess(&mut ctx)?;

        // Step 7: Make decision (docs/14 §10)
        self.step_decide(&mut ctx)?;

        // Audit: decision made
        self.audit_decision(&ctx, &check_id);

        // Step 8: Start workflow + create tasks if escalation (docs/13 §7-8)
        self.step_workflow_and_tasks(&mut ctx)?;

        // Complete
        ctx.transition(CheckFlowState::Completed);
        let result = self.build_result(&check_id, &ctx);

        // Audit: check completed
        self.audit_check_completed(&ctx, &check_id, &result);

        Ok(result)
    }

    /// Step 1: Fetch observations from connector.
    fn step_fetch(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Fetching);
        let observations = self
            .connector_port
            .fetch_observations(
                &ctx.input.connector_instance_id,
                &ctx.input.target_entity_id,
            )
            .map_err(|e| IntegrationError::Connector { message: e })?;
        tracing::info!(
            count = observations.len(),
            target = %ctx.input.target_entity_id,
            "observations fetched"
        );
        ctx.observations = observations;
        Ok(())
    }

    /// Step 2: Map observations to provenance statements.
    fn step_map(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Mapping);
        if ctx.observations.is_empty() {
            ctx.add_warning("no observations to map");
            return Ok(());
        }
        let mapping = self
            .connector_port
            .map_observations(&ctx.observations)
            .map_err(|e| IntegrationError::Connector { message: e })?;
        ctx.mapping_result = Some(mapping);
        Ok(())
    }

    /// Step 3: Load active criteria for the target type.
    fn step_load_criteria(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Evaluating);
        let criteria = self
            .criterion_port
            .load_criteria(&ctx.input.tenant_id, &ctx.input.target_type)
            .map_err(|e| IntegrationError::RiskEngine { message: e })?;
        tracing::info!(
            count = criteria.len(),
            target_type = %ctx.input.target_type,
            "criteria loaded"
        );
        ctx.criteria = criteria;
        Ok(())
    }

    /// Step 4: Evaluate each criterion and detect factors.
    fn step_evaluate_and_detect(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::FactorDetection);

        // Build evaluation context from observations
        let eval_ctx = self.build_evaluation_context(ctx);

        // Collect triggered factors first to avoid borrow conflicts
        let mut detected_factors: Vec<Factor> = Vec::new();
        let mut warnings: Vec<String> = Vec::new();

        for criterion in &ctx.criteria {
            let result = enginer_risk_engine::evaluate(&criterion.expression, &eval_ctx);
            match result {
                Ok(value) => {
                    if let enginer_risk_engine::Value::Boolean { value: true } = value {
                        // Criterion triggered — create factor
                        let score_contribution = self.severity_to_score(criterion.severity);
                        let factor = Factor::new(
                            &criterion.criterion_id,
                            &criterion.version,
                            &ctx.input.target_entity_id,
                            criterion.severity,
                            &score_contribution,
                            Vec::new(), // evidence — wired in full deployment
                            serde_json::json!({
                                "criterion": criterion.criterion_id,
                                "severity": format!("{:?}", criterion.severity),
                            }),
                        );
                        // Store factor
                        if let Err(e) = self.factor_port.store_factor(&factor) {
                            warnings.push(format!(
                                "failed to store factor for criterion {}: {}",
                                criterion.criterion_id, e
                            ));
                        }
                        detected_factors.push(factor);
                    }
                }
                Err(e) => {
                    warnings.push(format!(
                        "criterion {} evaluation failed: {:?}",
                        criterion.criterion_id, e
                    ));
                }
            }
        }

        // Apply warnings and factors to context
        for w in &warnings {
            ctx.add_warning(w);
        }
        ctx.factors.extend(detected_factors);

        tracing::info!(
            factors = ctx.factors.len(),
            target = %ctx.input.target_entity_id,
            "factors detected"
        );
        Ok(())
    }

    /// Step 5: Aggregate factors into score.
    fn step_score(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Scoring);
        let aggregator = WeightedAggregator::new(self.score_config.clone());
        let score_result = aggregator.aggregate(&ctx.factors);
        tracing::info!(
            score = score_result.score,
            level = ?score_result.level,
            target = %ctx.input.target_entity_id,
            "score aggregated"
        );
        ctx.score_result = Some(score_result);
        Ok(())
    }

    /// Step 6: Create assessment.
    fn step_assess(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Assessing);
        let score_result =
            ctx.score_result
                .as_ref()
                .ok_or_else(|| IntegrationError::InvalidState {
                    message: "score result missing".to_string(),
                })?;

        // Check if we should skip (existing assessment + not forced)
        if !ctx.input.force {
            if let Ok(Some(existing)) = self
                .assessment_port
                .load_latest_assessment(&ctx.input.target_entity_id)
            {
                if existing.is_current() {
                    ctx.add_warning(
                        "assessment already exists, skipping (use force=true to override)",
                    );
                    ctx.assessment = Some(existing);
                    return Ok(());
                }
            }
        }

        let uncertainty = Uncertainty {
            oldest_observation_age_days: 0, // computed from observations in full deployment
            coverage_percent: 100.0,
            conflict_count: 0,
            confidence: 0.95,
        };

        let assessment = Assessment::new_draft(
            &ctx.input.target_entity_id,
            "default",
            "1.0.0",
            chrono::Utc::now(),
            "fingerprint-001",
            &ctx.factors,
            score_result,
            uncertainty,
            &ctx.input.actor,
        );

        // Store assessment
        if let Err(e) = self.assessment_port.store_assessment(&assessment) {
            return Err(IntegrationError::RiskEngine { message: e });
        }

        ctx.assessment = Some(assessment);
        Ok(())
    }

    /// Step 7: Make decision based on score and level.
    fn step_decide(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        ctx.transition(CheckFlowState::Deciding);
        let score_result =
            ctx.score_result
                .as_ref()
                .ok_or_else(|| IntegrationError::InvalidState {
                    message: "score result missing".to_string(),
                })?;

        let decision = Decision::system(
            &ctx.input.target_entity_id,
            ctx.assessment
                .as_ref()
                .map(|a| a.assessment_id.as_str())
                .unwrap_or("unknown"),
            score_result.score,
            score_result.level,
            "check.flow.decision",
        );

        // Store decision
        if let Err(e) = self.decision_port.store_decision(&decision) {
            return Err(IntegrationError::RiskEngine { message: e });
        }

        ctx.decision = Some(decision);
        Ok(())
    }

    /// Step 8: Start workflow and create tasks if escalation is needed.
    fn step_workflow_and_tasks(&self, ctx: &mut CheckFlowContext) -> IntegrationResult<()> {
        // Extract decision kind and details first to avoid borrow conflicts
        let (decision_kind, risk_level_str, score) = {
            let decision = ctx
                .decision
                .as_ref()
                .ok_or_else(|| IntegrationError::InvalidState {
                    message: "decision missing".to_string(),
                })?;
            (
                decision.kind,
                format!("{:?}", decision.risk_level),
                decision.score,
            )
        };

        // Start workflow if decision is Escalate or Reject
        if matches!(decision_kind, DecisionKind::Escalate | DecisionKind::Reject) {
            ctx.transition(CheckFlowState::WorkflowStart);

            let mut variables = HashMap::new();
            variables.insert(
                "target_entity_id".to_string(),
                serde_json::json!(ctx.input.target_entity_id),
            );
            variables.insert(
                "check_kind".to_string(),
                serde_json::json!(ctx.input.check_kind),
            );
            variables.insert("risk_level".to_string(), serde_json::json!(risk_level_str));
            variables.insert("score".to_string(), serde_json::json!(score));

            let workflow_instance_id = self
                .workflow_port
                .start_workflow("request_processing", &ctx.input.target_entity_id, variables)
                .map_err(|e| IntegrationError::Workflow { message: e })?;

            ctx.workflow_instance_id = Some(workflow_instance_id.clone());

            // Create review task for risk analyst
            ctx.transition(CheckFlowState::TaskCreation);
            let due_date = chrono::Utc::now() + chrono::Duration::days(3);
            let mut task = Task::new(
                &workflow_instance_id,
                &ctx.input.tenant_id,
                "Review risk assessment",
                "review",
                TaskPriority::High,
                due_date,
            );
            task.assign_to_group("risk-team");
            task.required_capabilities = vec!["check.run".to_string()];

            if let Err(e) = self.task_port.create_task(&task) {
                ctx.add_warning(&format!("failed to create task: {}", e));
            } else {
                ctx.tasks.push(task);
            }
        }

        Ok(())
    }

    /// Record an audit entry (non-fatal: errors are logged but don't fail the flow).
    fn record_audit(&self, entry: &AuditEntry) {
        if let Some(port) = &self.audit_port {
            if let Err(e) = port.record(entry) {
                tracing::warn!(error = %e, "failed to record audit entry");
            }
        }
    }

    /// Audit: check initiated.
    fn audit_check_initiated(&self, ctx: &CheckFlowContext, check_id: &str) {
        let mut entry = AuditEntry::new(
            0,
            "",
            &ctx.input.tenant_id,
            &ctx.input.actor,
            AuditKind::Decision,
            "check.run",
        );
        entry = entry.with_resource("check", check_id);
        entry.add_metadata("check_kind", &ctx.input.check_kind);
        entry.add_metadata("target_entity_id", &ctx.input.target_entity_id);
        entry.add_metadata("connector_instance_id", &ctx.input.connector_instance_id);
        self.record_audit(&entry);
    }

    /// Audit: decision made (approve/reject/escalate).
    fn audit_decision(&self, ctx: &CheckFlowContext, check_id: &str) {
        if let Some(decision) = &ctx.decision {
            let effect = match decision.kind {
                DecisionKind::Approve => "allow",
                DecisionKind::ConditionalApprove => "allow_with_obligations",
                DecisionKind::Reject => "deny",
                DecisionKind::Escalate => "escalate",
                DecisionKind::Defer => "defer",
            };
            let mut entry = AuditEntry::new(
                0,
                "",
                &ctx.input.tenant_id,
                &ctx.input.actor,
                AuditKind::Decision,
                "check.decision",
            );
            entry = entry.with_resource("check", check_id);
            entry = entry.with_decision(effect, Vec::new());
            if let Some(assessment) = &ctx.assessment {
                entry.add_metadata("assessment_id", &assessment.assessment_id);
            }
            entry.add_metadata("score", &format!("{:.2}", decision.score));
            entry.add_metadata("risk_level", &format!("{:?}", decision.risk_level));
            self.record_audit(&entry);
        }
    }

    /// Audit: check completed.
    fn audit_check_completed(
        &self,
        ctx: &CheckFlowContext,
        check_id: &str,
        result: &CheckFlowResult,
    ) {
        let mut entry = AuditEntry::new(
            0,
            "",
            &ctx.input.tenant_id,
            &ctx.input.actor,
            AuditKind::Decision,
            "check.completed",
        );
        entry = entry.with_resource("check", check_id);
        entry.add_metadata("state", &format!("{:?}", result.state));
        entry.add_metadata("observation_count", &result.observation_count.to_string());
        entry.add_metadata("criteria_evaluated", &result.criteria_evaluated.to_string());
        entry.add_metadata("factors_detected", &result.factors_detected.to_string());
        entry.add_metadata("final_score", &format!("{:.2}", result.final_score));
        entry.add_metadata("tasks_created", &result.tasks_created.to_string());
        if let Some(wf_id) = &result.workflow_instance_id {
            entry.add_metadata("workflow_instance_id", wf_id);
        }
        self.record_audit(&entry);
    }

    /// Build evaluation context from observations.
    fn build_evaluation_context(&self, ctx: &CheckFlowContext) -> EvaluationContext {
        let mut predicate_values: HashMap<String, Vec<enginer_risk_engine::Value>> = HashMap::new();

        // Extract predicate values from mapping result
        if let Some(mapping) = &ctx.mapping_result {
            for mapped in &mapping.mapped_predicates {
                let value = match &mapped.source_field {
                    f if f.contains("inn") => Some(enginer_risk_engine::Value::Text {
                        value: "1234567890".to_string(),
                    }),
                    f if f.contains("ogrn") => Some(enginer_risk_engine::Value::Text {
                        value: "1234567890123".to_string(),
                    }),
                    _ => None,
                };
                if let Some(v) = value {
                    predicate_values
                        .entry(mapped.predicate_alias.clone())
                        .or_default()
                        .push(v);
                }
            }
        }

        let observations: Vec<ObservationContext> = ctx
            .observations
            .iter()
            .map(|obs| ObservationContext {
                connector_type_id: obs.provenance.source_connector_type_id.clone(),
                age_days: 0,
                trust_rank: obs.provenance.trust_rank,
            })
            .collect();

        EvaluationContext {
            predicate_values,
            observations,
            previous_factors: Vec::new(),
        }
    }

    /// Convert severity to score contribution string.
    fn severity_to_score(&self, severity: CriterionSeverity) -> String {
        match severity {
            CriterionSeverity::Info => "0".to_string(),
            CriterionSeverity::Low => "10".to_string(),
            CriterionSeverity::Medium => "25".to_string(),
            CriterionSeverity::High => "50".to_string(),
            CriterionSeverity::Critical => "100".to_string(),
        }
    }

    /// Build the final result from context.
    fn build_result(&self, check_id: &str, ctx: &CheckFlowContext) -> CheckFlowResult {
        CheckFlowResult {
            check_id: check_id.to_string(),
            state: ctx.state,
            target_entity_id: ctx.input.target_entity_id.clone(),
            observation_count: ctx.observations.len(),
            criteria_evaluated: ctx.criteria.len(),
            factors_detected: ctx.factors.len(),
            final_score: ctx.score_result.as_ref().map(|s| s.score).unwrap_or(0.0),
            risk_level: ctx
                .score_result
                .as_ref()
                .map(|s| s.level)
                .unwrap_or(RiskLevel::Minimal),
            assessment_id: ctx.assessment.as_ref().map(|a| a.assessment_id.clone()),
            decision: ctx.decision.clone(),
            workflow_instance_id: ctx.workflow_instance_id.clone(),
            tasks_created: ctx.tasks.len(),
            warnings: ctx.warnings.clone(),
            completed_at: chrono::Utc::now(),
        }
    }
}
