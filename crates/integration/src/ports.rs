//! Ports: traits for the check flow orchestrator (docs/13 §8, docs/14 §4-12).
//!
//! Each port is a trait that the persistence/adapter layer implements.
//! The orchestrator is pure — it coordinates the flow without knowing
//! how data is stored or how connectors are called.

use enginer_connectors::{MappingResult, SourceObservation};
use enginer_risk_engine::{Assessment, Criterion, Decision, Factor, ScoreResult};
use enginer_security::{AuditEntry, AuditKind};
use enginer_workflow::{StateMachine, Task};
use std::collections::HashMap;

/// Port for loading criteria (docs/14 §8).
pub trait CriterionPort: Send + Sync {
    /// Load active criteria for a tenant and target type.
    fn load_criteria(
        &self,
        tenant_id: &str,
        target_type: &str,
    ) -> Result<Vec<Criterion>, String>;
}

/// Port for calling connectors (docs/14 §5-6).
pub trait ConnectorPort: Send + Sync {
    /// Fetch observations from a connector for a target entity.
    fn fetch_observations(
        &self,
        connector_instance_id: &str,
        target_entity_id: &str,
    ) -> Result<Vec<SourceObservation>, String>;

    /// Map observations to provenance statements.
    fn map_observations(
        &self,
        observations: &[SourceObservation],
    ) -> Result<MappingResult, String>;
}

/// Port for storing and retrieving factors (docs/14 §9).
pub trait FactorPort: Send + Sync {
    /// Store a detected factor (upsert by dedup key).
    fn store_factor(&self, factor: &Factor) -> Result<(), String>;

    /// Load active factors for a target entity.
    fn load_factors(&self, target_entity_id: &str) -> Result<Vec<Factor>, String>;

    /// Resolve a factor (condition no longer met).
    fn resolve_factor(&self, factor_id: &str) -> Result<(), String>;
}

/// Port for storing and retrieving assessments (docs/14 §10).
pub trait AssessmentPort: Send + Sync {
    /// Store an assessment.
    fn store_assessment(&self, assessment: &Assessment) -> Result<(), String>;

    /// Load the latest assessment for a target entity.
    fn load_latest_assessment(
        &self,
        target_entity_id: &str,
    ) -> Result<Option<Assessment>, String>;
}

/// Port for storing decisions (docs/14 §10).
pub trait DecisionPort: Send + Sync {
    /// Store a decision.
    fn store_decision(&self, decision: &Decision) -> Result<(), String>;

    /// Load the latest decision for a target entity.
    fn load_latest_decision(
        &self,
        target_entity_id: &str,
    ) -> Result<Option<Decision>, String>;
}

/// Port for workflow operations (docs/13 §7).
pub trait WorkflowPort: Send + Sync {
    /// Start a new workflow instance. Returns the instance ID.
    fn start_workflow(
        &self,
        definition_id: &str,
        target_entity_id: &str,
        variables: HashMap<String, serde_json::Value>,
    ) -> Result<String, String>;

    /// Transition a workflow.
    fn transition_workflow(
        &self,
        instance_id: &str,
        to: &str,
        trigger: &str,
        actor: &str,
    ) -> Result<(), String>;

    /// Get workflow instance ID (returns the instance_id if it exists).
    fn get_workflow(&self, instance_id: &str) -> Result<Option<String>, String>;
}

/// Port for task operations (docs/13 §8).
pub trait TaskPort: Send + Sync {
    /// Create a task.
    fn create_task(&self, task: &Task) -> Result<(), String>;

    /// Load tasks for a workflow instance.
    fn load_tasks(&self, workflow_instance_id: &str) -> Result<Vec<Task>, String>;

    /// Complete a task.
    fn complete_task(&self, task_id: &str) -> Result<(), String>;
}

/// Port for audit logging (docs/08 §11).
///
/// The orchestrator emits audit events at key points in the check flow:
/// - `decision` when a decision is made (approve/reject/escalate);
/// - `denied_request` if the flow fails due to a denied capability;
/// - `bulk_action` for the overall check execution.
pub trait AuditPort: Send + Sync {
    /// Record an audit entry. Errors are non-fatal (logged but don't
    /// fail the check flow).
    fn record(&self, entry: &AuditEntry) -> Result<(), String>;
}
