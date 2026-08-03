//! Timers and scheduler (docs/13 §6, §12).
//!
//! Schedule stores IANA time zone, calendar expression, next occurrence,
//! misfire policy, concurrency policy and owner. DST handling for
//! nonexistent/repeated local times. Business calendar versioned per
//! tenant/location (docs/13 §12).

use serde::{Deserialize, Serialize};

/// Timer kind (docs/13 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerKind {
    /// SLA timer: fires when SLA is breached.
    Sla,
    /// Escalation timer: fires to escalate a task.
    Escalation,
    /// Reminder timer: fires before due date.
    Reminder,
    /// Monitoring timer: fires on schedule for periodic checks.
    Monitor,
    /// Timeout timer: fires when waiting too long.
    Timeout,
}

/// Misfire policy (docs/13 §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MisfirePolicy {
    /// Run once immediately if misfired.
    RunOnce,
    /// Skip the misfired occurrence.
    Skip,
    /// Run next occurrence.
    RunNext,
}

/// Schedule configuration (docs/13 §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScheduleConfig {
    /// IANA time zone (e.g. "Europe/Moscow").
    pub timezone: String,
    /// Calendar expression (cron-like or simple interval).
    pub calendar_expression: String,
    /// Misfire policy.
    pub misfire_policy: MisfirePolicy,
    /// Concurrency policy: "allow", "skip_if_running", "queue".
    pub concurrency_policy: String,
    /// Owner (user or team responsible).
    pub owner: String,
    /// Quiet period in seconds (suppress rapid re-triggers).
    pub quiet_period_seconds: u32,
}

/// A schedule (docs/13 §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    /// Schedule ID (UUID).
    pub schedule_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Target entity public ID (if monitoring a specific entity).
    pub target_entity_id: Option<String>,
    /// Target query (if monitoring a set of entities).
    pub target_query: Option<String>,
    /// Schedule configuration.
    pub config: ScheduleConfig,
    /// Next occurrence (UTC).
    pub next_run: chrono::DateTime<chrono::Utc>,
    /// Previous occurrence (UTC).
    pub last_run: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether the schedule is active.
    pub active: bool,
    /// Legal basis for monitoring.
    pub legal_basis: String,
    /// Created at.
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl Schedule {
    /// Create a new active schedule.
    pub fn new(
        tenant_id: &str,
        config: ScheduleConfig,
        next_run: chrono::DateTime<chrono::Utc>,
        legal_basis: &str,
    ) -> Self {
        Self {
            schedule_id: uuid::Uuid::now_v7().to_string(),
            tenant_id: tenant_id.to_string(),
            target_entity_id: None,
            target_query: None,
            config,
            next_run,
            last_run: None,
            active: true,
            legal_basis: legal_basis.to_string(),
            created_at: chrono::Utc::now(),
        }
    }

    /// Mark as run (update last_run, advance next_run).
    pub fn mark_run(&mut self, next_run: chrono::DateTime<chrono::Utc>) {
        self.last_run = Some(chrono::Utc::now());
        self.next_run = next_run;
    }

    /// Check if the schedule is due (current time >= next_run).
    pub fn is_due(&self) -> bool {
        self.active && chrono::Utc::now() >= self.next_run
    }

    /// Deactivate the schedule.
    pub fn deactivate(&mut self) {
        self.active = false;
    }

    /// Reactivate the schedule.
    pub fn reactivate(&mut self) {
        self.active = true;
    }
}

/// An escalation timer (docs/13 §7, §12).
///
/// Fires when a task is not completed within the SLA window, escalating
/// to a higher authority or triggering a reaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EscalationTimer {
    /// Timer ID (UUID).
    pub timer_id: String,
    /// Workflow instance ID.
    pub workflow_instance_id: String,
    /// Task ID (if escalating a specific task).
    pub task_id: Option<String>,
    /// Timer kind.
    pub kind: TimerKind,
    /// When the timer fires.
    pub fire_at: chrono::DateTime<chrono::Utc>,
    /// Escalation target (user or group ID).
    pub escalation_target: String,
    /// Whether the timer has fired.
    pub fired: bool,
    /// Whether the timer was cancelled.
    pub cancelled: bool,
    /// Business calendar version used to compute fire_at.
    pub calendar_version: Option<String>,
}

impl EscalationTimer {
    /// Create a new escalation timer.
    pub fn new(
        workflow_instance_id: &str,
        kind: TimerKind,
        fire_at: chrono::DateTime<chrono::Utc>,
        escalation_target: &str,
    ) -> Self {
        Self {
            timer_id: uuid::Uuid::now_v7().to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            task_id: None,
            kind,
            fire_at,
            escalation_target: escalation_target.to_string(),
            fired: false,
            cancelled: false,
            calendar_version: None,
        }
    }

    /// Check if the timer is due to fire.
    pub fn is_due(&self) -> bool {
        !self.fired && !self.cancelled && chrono::Utc::now() >= self.fire_at
    }

    /// Mark the timer as fired.
    pub fn fire(&mut self) {
        self.fired = true;
    }

    /// Cancel the timer.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }

    /// Link the timer to a specific task.
    pub fn link_to_task(&mut self, task_id: &str) {
        self.task_id = Some(task_id.to_string());
    }
}
