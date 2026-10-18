//! Task entity (docs/13 §7-8).
//!
//! Tasks are assigned to users/groups as part of workflow execution.
//! Assignment strategy, priority, due date, SLA.

use serde::{Deserialize, Serialize};

/// Task status (docs/13 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task created, not yet assigned.
    Pending,
    /// Task assigned to a user/group.
    Assigned,
    /// Task accepted and being worked on.
    InProgress,
    /// Task completed.
    Completed,
    /// Task cancelled.
    Cancelled,
    /// Task overdue (past due date).
    Overdue,
    /// Task escalated.
    Escalated,
}

/// Task priority (docs/13 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    /// Low priority.
    Low,
    /// Normal priority.
    Normal,
    /// High priority.
    High,
    /// Urgent priority.
    Urgent,
}

impl TaskPriority {
    /// Get numeric priority value (higher = more important).
    pub fn value(self) -> i32 {
        match self {
            Self::Low => 1,
            Self::Normal => 5,
            Self::High => 10,
            Self::Urgent => 20,
        }
    }
}

/// Assignment strategy (docs/13 §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStrategy {
    /// Assign to a specific user.
    Direct,
    /// Assign to a group (round-robin or claim).
    Group,
    /// Auto-assign based on rules.
    Auto,
    /// Self-assign (user claims the task).
    SelfAssign,
}

/// A workflow task (docs/13 §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    /// Task ID (UUID).
    pub task_id: String,
    /// Workflow instance ID.
    pub workflow_instance_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Task title.
    pub title: String,
    /// Task description (i18n key).
    pub description_key: String,
    /// Task kind (e.g. "review", "approval", "check", "data_entry").
    pub kind: String,
    /// Status.
    pub status: TaskStatus,
    /// Priority.
    pub priority: TaskPriority,
    /// Assignment strategy.
    pub assignment_strategy: AssignmentStrategy,
    /// Assigned user ID (if Direct).
    pub assignee_id: Option<String>,
    /// Assigned group ID (if Group).
    pub group_id: Option<String>,
    /// Due date (computed once with business calendar, docs/13 §12).
    pub due_date: chrono::DateTime<chrono::Utc>,
    /// Business calendar version used to compute due_date.
    pub calendar_version: Option<String>,
    /// Created at.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Completed at (if completed).
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Task variables (for form rendering).
    pub variables: serde_json::Value,
    /// Required capabilities (docs/08 §2).
    pub required_capabilities: Vec<String>,
}

impl Task {
    /// Create a new pending task.
    pub fn new(
        workflow_instance_id: &str,
        tenant_id: &str,
        title: &str,
        kind: &str,
        priority: TaskPriority,
        due_date: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        Self {
            task_id: uuid::Uuid::now_v7().to_string(),
            workflow_instance_id: workflow_instance_id.to_string(),
            tenant_id: tenant_id.to_string(),
            title: title.to_string(),
            description_key: format!("task.{}.description", kind),
            kind: kind.to_string(),
            status: TaskStatus::Pending,
            priority,
            assignment_strategy: AssignmentStrategy::Auto,
            assignee_id: None,
            group_id: None,
            due_date,
            calendar_version: None,
            created_at: chrono::Utc::now(),
            completed_at: None,
            variables: serde_json::json!({}),
            required_capabilities: Vec::new(),
        }
    }

    /// Assign to a specific user.
    pub fn assign_to_user(&mut self, user_id: &str) {
        self.assignee_id = Some(user_id.to_string());
        self.assignment_strategy = AssignmentStrategy::Direct;
        self.status = TaskStatus::Assigned;
    }

    /// Assign to a group.
    pub fn assign_to_group(&mut self, group_id: &str) {
        self.group_id = Some(group_id.to_string());
        self.assignment_strategy = AssignmentStrategy::Group;
        self.status = TaskStatus::Assigned;
    }

    /// Accept the task (user starts working on it).
    pub fn accept(&mut self) -> Result<(), String> {
        if self.status != TaskStatus::Assigned {
            return Err(format!("cannot accept task in status {:?}", self.status));
        }
        self.status = TaskStatus::InProgress;
        Ok(())
    }

    /// Complete the task.
    pub fn complete(&mut self) -> Result<(), String> {
        if self.status != TaskStatus::InProgress {
            return Err(format!("cannot complete task in status {:?}", self.status));
        }
        self.status = TaskStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
        Ok(())
    }

    /// Cancel the task.
    pub fn cancel(&mut self) -> Result<(), String> {
        if self.status.is_terminal() {
            return Err(format!(
                "cannot cancel task in terminal status {:?}",
                self.status
            ));
        }
        self.status = TaskStatus::Cancelled;
        Ok(())
    }

    /// Mark as overdue.
    pub fn mark_overdue(&mut self) {
        if self.status == TaskStatus::Pending
            || self.status == TaskStatus::Assigned
            || self.status == TaskStatus::InProgress
        {
            self.status = TaskStatus::Overdue;
        }
    }

    /// Escalate the task.
    pub fn escalate(&mut self) {
        self.status = TaskStatus::Escalated;
    }

    /// Check if the task is overdue (past due date and not terminal).
    pub fn is_overdue(&self) -> bool {
        !self.status.is_terminal() && chrono::Utc::now() > self.due_date
    }

    /// Check if the task is terminal.
    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }
}

impl TaskStatus {
    /// Check if this is a terminal status.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled)
    }
}
