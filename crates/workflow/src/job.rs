//! Job state machine (docs/13 §4).
//!
//! queued → leased → running → waiting_external/waiting_user →
//! succeeded/partially_succeeded/failed/cancelled.
//!
//! Job stores progress numerator/denominator, phase, heartbeat, attempts,
//! deadline, lease, resumable cursor, result/error artifact. Cancellation
//! is cooperative; irreversible step reports cannot_cancel_after.

use serde::{Deserialize, Serialize};

/// Job state (docs/13 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    /// Queued, waiting for a worker lease.
    Queued,
    /// Leased by a worker, not yet running.
    Leased,
    /// Running on a worker.
    Running,
    /// Waiting for external system response.
    WaitingExternal,
    /// Waiting for user input.
    WaitingUser,
    /// Completed successfully.
    Succeeded,
    /// Completed with partial success.
    PartiallySucceeded,
    /// Failed.
    Failed,
    /// Cancelled (cooperative).
    Cancelled,
}

impl JobState {
    /// Check if this is a terminal state.
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::PartiallySucceeded | Self::Failed | Self::Cancelled
        )
    }

    /// Check if this state allows cancellation.
    pub fn is_cancellable(self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Leased | Self::Running | Self::WaitingExternal | Self::WaitingUser
        )
    }
}

/// Job status (progress + phase + heartbeat).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobStatus {
    /// Current phase name.
    pub phase: String,
    /// Progress numerator (completed items).
    pub progress_numerator: u64,
    /// Progress denominator (total items).
    pub progress_denominator: u64,
    /// Last heartbeat timestamp.
    pub last_heartbeat: Option<chrono::DateTime<chrono::Utc>>,
    /// Number of attempts.
    pub attempts: u32,
    /// Job deadline.
    pub deadline: chrono::DateTime<chrono::Utc>,
    /// Current lease owner (worker ID).
    pub lease_owner: Option<String>,
    /// Lease expiry time.
    pub lease_expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Resumable cursor (for checkpoint/restart).
    pub cursor: Option<String>,
    /// Result artifact (if succeeded).
    pub result: Option<serde_json::Value>,
    /// Error artifact (if failed).
    pub error: Option<String>,
    /// Whether an irreversible step has started (cannot cancel after).
    pub irreversible_step_started: bool,
}

/// A job instance (docs/13 §4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    /// Job ID (UUID).
    pub job_id: String,
    /// Tenant ID.
    pub tenant_id: String,
    /// Job type (e.g. "check", "import", "export", "report", "projection").
    pub job_type: String,
    /// Workload class (docs/13 §5).
    pub workload_class: String,
    /// Current state.
    pub state: JobState,
    /// Job status.
    pub status: JobStatus,
    /// Created at.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Updated at.
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Priority (higher = more important).
    pub priority: i32,
    /// Correlation ID (for tracing).
    pub correlation_id: Option<String>,
    /// Causation ID (event that triggered this job).
    pub causation_id: Option<String>,
}

impl Job {
    /// Create a new queued job.
    pub fn new(
        tenant_id: &str,
        job_type: &str,
        workload_class: &str,
        deadline: chrono::DateTime<chrono::Utc>,
    ) -> Self {
        let now = chrono::Utc::now();
        Self {
            job_id: uuid::Uuid::now_v7().to_string(),
            tenant_id: tenant_id.to_string(),
            job_type: job_type.to_string(),
            workload_class: workload_class.to_string(),
            state: JobState::Queued,
            status: JobStatus {
                phase: "init".to_string(),
                progress_numerator: 0,
                progress_denominator: 0,
                last_heartbeat: None,
                attempts: 0,
                deadline,
                lease_owner: None,
                lease_expires_at: None,
                cursor: None,
                result: None,
                error: None,
                irreversible_step_started: false,
            },
            created_at: now,
            updated_at: now,
            priority: 0,
            correlation_id: None,
            causation_id: None,
        }
    }

    /// Lease the job to a worker.
    pub fn lease(
        &mut self,
        worker_id: &str,
        lease_duration: chrono::Duration,
    ) -> Result<(), String> {
        if self.state != JobState::Queued && self.state != JobState::Leased {
            return Err(format!("cannot lease job in state {:?}", self.state));
        }
        self.state = JobState::Leased;
        self.status.lease_owner = Some(worker_id.to_string());
        self.status.lease_expires_at = Some(chrono::Utc::now() + lease_duration);
        self.status.attempts += 1;
        self.touch();
        Ok(())
    }

    /// Start running the job.
    pub fn start(&mut self) -> Result<(), String> {
        if self.state != JobState::Leased {
            return Err(format!("cannot start job in state {:?}", self.state));
        }
        self.state = JobState::Running;
        self.status.phase = "running".to_string();
        self.heartbeat();
        Ok(())
    }

    /// Update heartbeat.
    pub fn heartbeat(&mut self) {
        self.status.last_heartbeat = Some(chrono::Utc::now());
        self.touch();
    }

    /// Update progress.
    pub fn update_progress(&mut self, numerator: u64, denominator: u64) {
        self.status.progress_numerator = numerator;
        self.status.progress_denominator = denominator;
        self.heartbeat();
    }

    /// Set cursor (for checkpoint/restart).
    pub fn set_cursor(&mut self, cursor: &str) {
        self.status.cursor = Some(cursor.to_string());
        self.heartbeat();
    }

    /// Mark as waiting for external system.
    pub fn wait_external(&mut self) -> Result<(), String> {
        if self.state != JobState::Running {
            return Err(format!("cannot wait_external from state {:?}", self.state));
        }
        self.state = JobState::WaitingExternal;
        self.touch();
        Ok(())
    }

    /// Mark as waiting for user input.
    pub fn wait_user(&mut self) -> Result<(), String> {
        if self.state != JobState::Running {
            return Err(format!("cannot wait_user from state {:?}", self.state));
        }
        self.state = JobState::WaitingUser;
        self.touch();
        Ok(())
    }

    /// Resume from waiting state.
    pub fn resume(&mut self) -> Result<(), String> {
        if self.state != JobState::WaitingExternal && self.state != JobState::WaitingUser {
            return Err(format!("cannot resume from state {:?}", self.state));
        }
        self.state = JobState::Running;
        self.heartbeat();
        Ok(())
    }

    /// Complete successfully.
    pub fn succeed(&mut self, result: serde_json::Value) -> Result<(), String> {
        if self.state != JobState::Running {
            return Err(format!("cannot succeed from state {:?}", self.state));
        }
        self.state = JobState::Succeeded;
        self.status.result = Some(result);
        self.status.phase = "completed".to_string();
        self.touch();
        Ok(())
    }

    /// Complete with partial success.
    pub fn partially_succeed(&mut self, result: serde_json::Value) -> Result<(), String> {
        if self.state != JobState::Running {
            return Err(format!(
                "cannot partially_succeed from state {:?}",
                self.state
            ));
        }
        self.state = JobState::PartiallySucceeded;
        self.status.result = Some(result);
        self.status.phase = "partial".to_string();
        self.touch();
        Ok(())
    }

    /// Fail the job.
    pub fn fail(&mut self, error: &str) -> Result<(), String> {
        if self.state != JobState::Running {
            return Err(format!("cannot fail from state {:?}", self.state));
        }
        self.state = JobState::Failed;
        self.status.error = Some(error.to_string());
        self.status.phase = "failed".to_string();
        self.touch();
        Ok(())
    }

    /// Cancel the job (cooperative, docs/13 §4).
    pub fn cancel(&mut self) -> Result<(), String> {
        if !self.state.is_cancellable() {
            return Err(format!(
                "cannot cancel job in terminal state {:?}",
                self.state
            ));
        }
        if self.status.irreversible_step_started {
            return Err("cannot_cancel_after: irreversible step has started".to_string());
        }
        self.state = JobState::Cancelled;
        self.status.phase = "cancelled".to_string();
        self.touch();
        Ok(())
    }

    /// Mark that an irreversible step has started.
    pub fn mark_irreversible(&mut self) {
        self.status.irreversible_step_started = true;
        self.touch();
    }

    /// Check if the lease has expired.
    pub fn is_lease_expired(&self) -> bool {
        self.status
            .lease_expires_at
            .map(|exp| chrono::Utc::now() > exp)
            .unwrap_or(false)
    }

    /// Check if the job is terminal.
    pub fn is_terminal(&self) -> bool {
        self.state.is_terminal()
    }

    /// Update the updated_at timestamp.
    fn touch(&mut self) {
        self.updated_at = chrono::Utc::now();
    }
}
