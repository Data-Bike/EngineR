//! Reactions: automatic actions triggered by events (docs/13 §10, §12).
//!
//! Allowed actions registry (docs/13 §10):
//! - create task/case;
//! - change category via command;
//! - start check/report;
//! - send e-mail/webhook/message;
//! - notify external system;
//! - request approval;
//! - escalate.
//!
//! Each action has schema, permission, idempotency, timeout, retry and
//! compensation. Arbitrary shell/script is forbidden (docs/13 §10).
//!
//! Criterion/assessment threshold can create task, alert, report,
//! re-check or outbound notification (docs/14 §12). Rule must have
//! cooldown/dedup key to prevent storm.

use serde::{Deserialize, Serialize};

/// Kind of action a reaction can trigger (docs/13 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Create a task or case.
    CreateTask,
    /// Change entity category via command.
    ChangeCategory,
    /// Start a check.
    StartCheck,
    /// Start a report generation.
    StartReport,
    /// Send an email.
    SendEmail,
    /// Send a webhook.
    SendWebhook,
    /// Send a message (in-app/push).
    SendMessage,
    /// Notify an external system.
    NotifyExternal,
    /// Request approval.
    RequestApproval,
    /// Escalate to a higher authority.
    Escalate,
}

/// Kind of trigger that fires a reaction (docs/13 §10, §14 §12).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionTriggerKind {
    /// Risk criterion triggered.
    CriterionTriggered,
    /// Assessment reached a threshold.
    AssessmentThreshold,
    /// Assessment changed.
    AssessmentChanged,
    /// Risk factor detected.
    FactorDetected,
    /// Risk factor resolved.
    FactorResolved,
    /// Task became overdue.
    TaskOverdue,
    /// Monitor detected a change.
    MonitorChangeDetected,
    /// Entity created.
    EntityCreated,
    /// Entity changed.
    EntityChanged,
    /// Entity retired.
    EntityRetired,
    /// Scheduled trigger.
    Schedule,
    /// Manual trigger.
    Manual,
}

/// A reaction trigger condition (docs/13 §10, §14 §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionTrigger {
    /// Trigger kind.
    pub kind: ReactionTriggerKind,
    /// Criterion ID (if CriterionTriggered).
    pub criterion_id: Option<String>,
    /// Threshold value (if AssessmentThreshold).
    pub threshold: Option<String>,
    /// Target type (if entity-based trigger).
    pub target_type: Option<String>,
}

/// A reaction configuration (docs/13 §10, §14 §12).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReactionConfig {
    /// Reaction ID.
    pub reaction_id: String,
    /// Reaction version.
    pub version: String,
    /// Trigger condition.
    pub trigger: ReactionTrigger,
    /// Action to execute.
    pub action: ActionKind,
    /// Action parameters (action-specific config).
    pub action_params: serde_json::Value,
    /// Required permission (docs/08 §2).
    pub required_permission: String,
    /// Idempotency key template (must include dedup, docs/14 §12).
    pub idempotency_key_template: String,
    /// Cooldown in seconds (prevent storm, docs/14 §12).
    pub cooldown_seconds: u32,
    /// Dedup key (prevent duplicate reactions, docs/14 §12).
    pub dedup_key: String,
    /// Timeout in seconds.
    pub timeout_seconds: u32,
    /// Max retry count.
    pub max_retries: u32,
    /// Compensation action (if the reaction fails).
    pub compensation: Option<String>,
    /// Whether this reaction requires workflow approval (docs/14 §12).
    pub requires_approval: bool,
    /// Whether the reaction is active.
    pub active: bool,
}

impl ReactionConfig {
    /// Create a new reaction configuration.
    pub fn new(
        reaction_id: &str,
        trigger: ReactionTrigger,
        action: ActionKind,
        dedup_key: &str,
    ) -> Self {
        Self {
            reaction_id: reaction_id.to_string(),
            version: "1.0.0".to_string(),
            trigger,
            action,
            action_params: serde_json::json!({}),
            required_permission: "reaction.execute".to_string(),
            idempotency_key_template: format!("{}:{{{{target}}}}:{{{{trigger}}}}", reaction_id),
            cooldown_seconds: 300, // 5 minutes default
            dedup_key: dedup_key.to_string(),
            timeout_seconds: 30,
            max_retries: 3,
            compensation: None,
            requires_approval: false,
            active: true,
        }
    }

    /// Check if the reaction can fire (cooldown check).
    pub fn can_fire(&self, last_fired_at: Option<chrono::DateTime<chrono::Utc>>) -> bool {
        if !self.active {
            return false;
        }
        if let Some(last) = last_fired_at {
            let cooldown = chrono::Duration::seconds(self.cooldown_seconds as i64);
            return chrono::Utc::now() >= last + cooldown;
        }
        true
    }
}

/// A reaction instance (fired reaction).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reaction {
    /// Reaction instance ID (UUID).
    pub instance_id: String,
    /// Reaction config ID.
    pub reaction_id: String,
    /// Reaction config version.
    pub version: String,
    /// Target entity public ID.
    pub target_entity_id: String,
    /// Trigger that fired.
    pub trigger: ReactionTrigger,
    /// Action that was executed.
    pub action: ActionKind,
    /// Idempotency key (resolved from template).
    pub idempotency_key: String,
    /// When the reaction fired.
    pub fired_at: chrono::DateTime<chrono::Utc>,
    /// Result (success/failure).
    pub result: ReactionResult,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Result of a reaction execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReactionResult {
    /// Reaction executed successfully.
    Success,
    /// Reaction failed.
    Failed,
    /// Reaction was skipped (cooldown or dedup).
    Skipped,
    /// Reaction is pending approval.
    PendingApproval,
}

impl Reaction {
    /// Create a new reaction instance.
    pub fn new(
        config: &ReactionConfig,
        target_entity_id: &str,
        trigger_context: &str,
    ) -> Self {
        let idempotency_key = config
            .idempotency_key_template
            .replace("{target}", target_entity_id)
            .replace("{trigger}", trigger_context);
        Self {
            instance_id: uuid::Uuid::now_v7().to_string(),
            reaction_id: config.reaction_id.clone(),
            version: config.version.clone(),
            target_entity_id: target_entity_id.to_string(),
            trigger: config.trigger.clone(),
            action: config.action,
            idempotency_key,
            fired_at: chrono::Utc::now(),
            result: ReactionResult::Success,
            error: None,
        }
    }

    /// Mark the reaction as failed.
    pub fn fail(&mut self, error: &str) {
        self.result = ReactionResult::Failed;
        self.error = Some(error.to_string());
    }

    /// Mark the reaction as skipped.
    pub fn skip(&mut self) {
        self.result = ReactionResult::Skipped;
    }

    /// Mark the reaction as pending approval.
    pub fn pending_approval(&mut self) {
        self.result = ReactionResult::PendingApproval;
    }
}
