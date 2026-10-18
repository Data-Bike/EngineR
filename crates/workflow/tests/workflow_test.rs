//! Workflow tests (docs/13 §4-12).

#![cfg(test)]

use enginer_workflow::{
    ActionKind, AssignmentStrategy, EscalationTimer, Job, JobState, MisfirePolicy, Reaction,
    ReactionConfig, ReactionTrigger, ReactionTriggerKind, Schedule, ScheduleConfig, State,
    StateMachine, StateMachineDefinition, StateTransition, Task, TaskPriority, TaskStatus,
    TimerKind,
};

// --- State machine tests ---

fn make_request_workflow() -> StateMachineDefinition {
    StateMachineDefinition {
        definition_id: "request_processing".to_string(),
        version: "1.0.0".to_string(),
        states: vec![
            State {
                name: "new".to_string(),
                terminal: false,
                initial: true,
            },
            State {
                name: "in_review".to_string(),
                terminal: false,
                initial: false,
            },
            State {
                name: "checks_running".to_string(),
                terminal: false,
                initial: false,
            },
            State {
                name: "assessment".to_string(),
                terminal: false,
                initial: false,
            },
            State {
                name: "decision".to_string(),
                terminal: false,
                initial: false,
            },
            State {
                name: "completed".to_string(),
                terminal: true,
                initial: false,
            },
            State {
                name: "cancelled".to_string(),
                terminal: true,
                initial: false,
            },
        ],
        transitions: vec![
            StateTransition {
                from: "new".to_string(),
                to: "in_review".to_string(),
                trigger: "start_review".to_string(),
                guard: None,
                actions: vec!["assign_reviewer".to_string()],
                requires_approval: false,
            },
            StateTransition {
                from: "in_review".to_string(),
                to: "checks_running".to_string(),
                trigger: "start_checks".to_string(),
                guard: None,
                actions: vec!["enqueue_checks".to_string()],
                requires_approval: false,
            },
            StateTransition {
                from: "checks_running".to_string(),
                to: "assessment".to_string(),
                trigger: "checks_completed".to_string(),
                guard: None,
                actions: vec!["run_assessment".to_string()],
                requires_approval: false,
            },
            StateTransition {
                from: "assessment".to_string(),
                to: "decision".to_string(),
                trigger: "assessment_done".to_string(),
                guard: None,
                actions: vec!["create_decision_task".to_string()],
                requires_approval: false,
            },
            StateTransition {
                from: "decision".to_string(),
                to: "completed".to_string(),
                trigger: "decision_made".to_string(),
                guard: None,
                actions: vec!["notify_initiator".to_string()],
                requires_approval: false,
            },
            StateTransition {
                from: "new".to_string(),
                to: "cancelled".to_string(),
                trigger: "cancel".to_string(),
                guard: None,
                actions: vec![],
                requires_approval: false,
            },
        ],
        timers: vec!["sla_timer".to_string()],
        assignment_strategy: "auto".to_string(),
        escalation: vec!["escalate_to_manager".to_string()],
        compensation: vec!["rollback_checks".to_string()],
    }
}

#[test]
fn state_machine_starts_in_initial_state() {
    let def = make_request_workflow();
    let sm = StateMachine::new(&def, "inst-001").unwrap();
    assert_eq!(sm.current_state, "new");
    assert!(!sm.terminated);
}

#[test]
fn state_machine_transition() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    let record = sm
        .transition(&def, "in_review", "start_review", "user-001")
        .unwrap();
    assert_eq!(sm.current_state, "in_review");
    assert_eq!(record.from, "new");
    assert_eq!(record.to, "in_review");
    assert_eq!(record.actor, "user-001");
    assert_eq!(sm.history.len(), 1);
}

#[test]
fn state_machine_invalid_transition() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    // No transition from "new" to "assessment"
    let result = sm.transition(&def, "assessment", "checks_completed", "user-001");
    assert!(result.is_err());
}

#[test]
fn state_machine_trigger_mismatch() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    let result = sm.transition(&def, "in_review", "wrong_trigger", "user-001");
    assert!(result.is_err());
}

#[test]
fn state_machine_reaches_terminal() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    sm.transition(&def, "in_review", "start_review", "user-001")
        .unwrap();
    sm.transition(&def, "checks_running", "start_checks", "user-001")
        .unwrap();
    sm.transition(&def, "assessment", "checks_completed", "user-001")
        .unwrap();
    sm.transition(&def, "decision", "assessment_done", "user-001")
        .unwrap();
    sm.transition(&def, "completed", "decision_made", "user-001")
        .unwrap();
    assert_eq!(sm.current_state, "completed");
    assert!(sm.terminated);
    assert!(sm.is_terminal());
}

#[test]
fn state_machine_cancel_from_new() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    sm.transition(&def, "cancelled", "cancel", "user-001")
        .unwrap();
    assert_eq!(sm.current_state, "cancelled");
    assert!(sm.terminated);
}

#[test]
fn state_machine_no_transition_after_terminal() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    sm.transition(&def, "cancelled", "cancel", "user-001")
        .unwrap();
    let result = sm.transition(&def, "in_review", "start_review", "user-001");
    assert!(result.is_err());
}

#[test]
fn state_machine_variables() {
    let def = make_request_workflow();
    let mut sm = StateMachine::new(&def, "inst-001").unwrap();
    sm.set_variable("request_id", serde_json::json!("req-001"));
    assert_eq!(
        sm.get_variable("request_id"),
        Some(&serde_json::json!("req-001"))
    );
    assert!(sm.get_variable("nonexistent").is_none());
}

#[test]
fn state_machine_definition_initial_state() {
    let def = make_request_workflow();
    let initial = def.initial_state().unwrap();
    assert_eq!(initial.name, "new");
    assert!(initial.initial);
}

#[test]
fn state_machine_definition_transitions_from() {
    let def = make_request_workflow();
    let transitions = def.transitions_from("new");
    assert_eq!(transitions.len(), 2); // start_review + cancel
}

// --- Job tests ---

#[test]
fn job_new_is_queued() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let job = Job::new("tenant-001", "check", "connectors", deadline);
    assert_eq!(job.state, JobState::Queued);
    assert!(!job.is_terminal());
}

#[test]
fn job_lifecycle_success() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    assert_eq!(job.state, JobState::Leased);
    assert_eq!(job.status.lease_owner.as_deref(), Some("worker-001"));
    job.start().unwrap();
    assert_eq!(job.state, JobState::Running);
    job.update_progress(5, 10);
    assert_eq!(job.status.progress_numerator, 5);
    job.succeed(serde_json::json!({"result": "ok"})).unwrap();
    assert_eq!(job.state, JobState::Succeeded);
    assert!(job.is_terminal());
    assert!(job.status.result.is_some());
}

#[test]
fn job_lifecycle_fail() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    job.start().unwrap();
    job.fail("network error").unwrap();
    assert_eq!(job.state, JobState::Failed);
    assert!(job.is_terminal());
    assert_eq!(job.status.error.as_deref(), Some("network error"));
}

#[test]
fn job_cancel() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.cancel().unwrap();
    assert_eq!(job.state, JobState::Cancelled);
    assert!(job.is_terminal());
}

#[test]
fn job_cancel_after_irreversible() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    job.start().unwrap();
    job.mark_irreversible();
    let result = job.cancel();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cannot_cancel_after"));
}

#[test]
fn job_wait_and_resume() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    job.start().unwrap();
    job.wait_external().unwrap();
    assert_eq!(job.state, JobState::WaitingExternal);
    job.resume().unwrap();
    assert_eq!(job.state, JobState::Running);
}

#[test]
fn job_wait_user_and_resume() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    job.start().unwrap();
    job.wait_user().unwrap();
    assert_eq!(job.state, JobState::WaitingUser);
    job.resume().unwrap();
    assert_eq!(job.state, JobState::Running);
}

#[test]
fn job_partial_success() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "import", "imports", deadline);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    job.start().unwrap();
    job.partially_succeed(serde_json::json!({"imported": 95, "failed": 5}))
        .unwrap();
    assert_eq!(job.state, JobState::PartiallySucceeded);
    assert!(job.is_terminal());
}

#[test]
fn job_lease_increments_attempts() {
    let deadline = chrono::Utc::now() + chrono::Duration::hours(1);
    let mut job = Job::new("tenant-001", "check", "connectors", deadline);
    assert_eq!(job.status.attempts, 0);
    job.lease("worker-001", chrono::Duration::minutes(10))
        .unwrap();
    assert_eq!(job.status.attempts, 1);
}

// --- Task tests ---

#[test]
fn task_new_is_pending() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let task = Task::new(
        "wf-001",
        "tenant-001",
        "Review request",
        "review",
        TaskPriority::Normal,
        due,
    );
    assert_eq!(task.status, TaskStatus::Pending);
    assert!(!task.is_terminal());
}

#[test]
fn task_assign_to_user() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::Normal,
        due,
    );
    task.assign_to_user("user-001");
    assert_eq!(task.status, TaskStatus::Assigned);
    assert_eq!(task.assignee_id.as_deref(), Some("user-001"));
    assert_eq!(task.assignment_strategy, AssignmentStrategy::Direct);
}

#[test]
fn task_assign_to_group() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::Normal,
        due,
    );
    task.assign_to_group("group-001");
    assert_eq!(task.status, TaskStatus::Assigned);
    assert_eq!(task.group_id.as_deref(), Some("group-001"));
    assert_eq!(task.assignment_strategy, AssignmentStrategy::Group);
}

#[test]
fn task_accept_and_complete() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::Normal,
        due,
    );
    task.assign_to_user("user-001");
    task.accept().unwrap();
    assert_eq!(task.status, TaskStatus::InProgress);
    task.complete().unwrap();
    assert_eq!(task.status, TaskStatus::Completed);
    assert!(task.is_terminal());
    assert!(task.completed_at.is_some());
}

#[test]
fn task_cancel() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::Normal,
        due,
    );
    task.cancel().unwrap();
    assert_eq!(task.status, TaskStatus::Cancelled);
    assert!(task.is_terminal());
}

#[test]
fn task_mark_overdue() {
    let due = chrono::Utc::now() - chrono::Duration::days(1); // past
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::Urgent,
        due,
    );
    assert!(task.is_overdue());
    task.mark_overdue();
    assert_eq!(task.status, TaskStatus::Overdue);
}

#[test]
fn task_escalate() {
    let due = chrono::Utc::now() + chrono::Duration::days(3);
    let mut task = Task::new(
        "wf-001",
        "tenant-001",
        "Review",
        "review",
        TaskPriority::High,
        due,
    );
    task.escalate();
    assert_eq!(task.status, TaskStatus::Escalated);
}

#[test]
fn task_priority_values() {
    assert_eq!(TaskPriority::Low.value(), 1);
    assert_eq!(TaskPriority::Normal.value(), 5);
    assert_eq!(TaskPriority::High.value(), 10);
    assert_eq!(TaskPriority::Urgent.value(), 20);
}

// --- Timer/Schedule tests ---

fn make_schedule_config() -> ScheduleConfig {
    ScheduleConfig {
        timezone: "Europe/Moscow".to_string(),
        calendar_expression: "0 9 * * MON".to_string(),
        misfire_policy: MisfirePolicy::RunOnce,
        concurrency_policy: "skip_if_running".to_string(),
        owner: "team-001".to_string(),
        quiet_period_seconds: 3600,
    }
}

#[test]
fn schedule_new_is_active() {
    let next_run = chrono::Utc::now() + chrono::Duration::days(7);
    let schedule = Schedule::new("tenant-001", make_schedule_config(), next_run, "kyc");
    assert!(schedule.active);
    assert!(!schedule.is_due());
}

#[test]
fn schedule_is_due() {
    let next_run = chrono::Utc::now() - chrono::Duration::hours(1); // past
    let schedule = Schedule::new("tenant-001", make_schedule_config(), next_run, "kyc");
    assert!(schedule.is_due());
}

#[test]
fn schedule_mark_run_advances_next() {
    let next_run = chrono::Utc::now() - chrono::Duration::hours(1);
    let mut schedule = Schedule::new("tenant-001", make_schedule_config(), next_run, "kyc");
    let new_next = chrono::Utc::now() + chrono::Duration::days(7);
    schedule.mark_run(new_next);
    assert!(schedule.last_run.is_some());
    assert_eq!(schedule.next_run, new_next);
    assert!(!schedule.is_due());
}

#[test]
fn schedule_deactivate_reactivate() {
    let next_run = chrono::Utc::now() + chrono::Duration::days(7);
    let mut schedule = Schedule::new("tenant-001", make_schedule_config(), next_run, "kyc");
    schedule.deactivate();
    assert!(!schedule.active);
    assert!(!schedule.is_due());
    schedule.reactivate();
    assert!(schedule.active);
}

#[test]
fn escalation_timer_is_due() {
    let fire_at = chrono::Utc::now() - chrono::Duration::minutes(5); // past
    let timer = EscalationTimer::new("wf-001", TimerKind::Escalation, fire_at, "manager-001");
    assert!(timer.is_due());
}

#[test]
fn escalation_timer_not_due() {
    let fire_at = chrono::Utc::now() + chrono::Duration::hours(1); // future
    let timer = EscalationTimer::new("wf-001", TimerKind::Escalation, fire_at, "manager-001");
    assert!(!timer.is_due());
}

#[test]
fn escalation_timer_fire() {
    let fire_at = chrono::Utc::now() - chrono::Duration::minutes(5);
    let mut timer = EscalationTimer::new("wf-001", TimerKind::Escalation, fire_at, "manager-001");
    timer.fire();
    assert!(timer.fired);
    assert!(!timer.is_due());
}

#[test]
fn escalation_timer_cancel() {
    let fire_at = chrono::Utc::now() - chrono::Duration::minutes(5);
    let mut timer = EscalationTimer::new("wf-001", TimerKind::Escalation, fire_at, "manager-001");
    timer.cancel();
    assert!(timer.cancelled);
    assert!(!timer.is_due());
}

#[test]
fn escalation_timer_link_to_task() {
    let fire_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let mut timer = EscalationTimer::new("wf-001", TimerKind::Sla, fire_at, "manager-001");
    timer.link_to_task("task-001");
    assert_eq!(timer.task_id.as_deref(), Some("task-001"));
}

// --- Reaction tests ---

#[test]
fn reaction_config_new() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    assert_eq!(config.reaction_id, "reaction-001");
    assert_eq!(config.action, ActionKind::CreateTask);
    assert!(config.active);
    assert_eq!(config.cooldown_seconds, 300);
}

#[test]
fn reaction_config_can_fire_no_history() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    assert!(config.can_fire(None));
}

#[test]
fn reaction_config_cannot_fire_during_cooldown() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    let last_fired = chrono::Utc::now();
    assert!(!config.can_fire(Some(last_fired)));
}

#[test]
fn reaction_config_can_fire_after_cooldown() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    let last_fired = chrono::Utc::now() - chrono::Duration::seconds(301);
    assert!(config.can_fire(Some(last_fired)));
}

#[test]
fn reaction_config_inactive_cannot_fire() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let mut config =
        ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    config.active = false;
    assert!(!config.can_fire(None));
}

#[test]
fn reaction_instance_new() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::AssessmentThreshold,
        criterion_id: None,
        threshold: Some("80.0".to_string()),
        target_type: Some("legal_entity".to_string()),
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::Escalate, "dedup-001");
    let reaction = Reaction::new(&config, "ent-001", "threshold_exceeded");
    assert_eq!(reaction.target_entity_id, "ent-001");
    assert_eq!(reaction.action, ActionKind::Escalate);
    assert!(!reaction.idempotency_key.is_empty());
    assert_eq!(
        reaction.result,
        enginer_workflow::reaction::ReactionResult::Success
    );
}

#[test]
fn reaction_instance_fail() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::SendEmail, "dedup-001");
    let mut reaction = Reaction::new(&config, "ent-001", "criterion_triggered");
    reaction.fail("SMTP error");
    assert_eq!(
        reaction.result,
        enginer_workflow::reaction::ReactionResult::Failed
    );
    assert_eq!(reaction.error.as_deref(), Some("SMTP error"));
}

#[test]
fn reaction_instance_skip() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::CriterionTriggered,
        criterion_id: Some("criterion-001".to_string()),
        threshold: None,
        target_type: None,
    };
    let config = ReactionConfig::new("reaction-001", trigger, ActionKind::CreateTask, "dedup-001");
    let mut reaction = Reaction::new(&config, "ent-001", "criterion_triggered");
    reaction.skip();
    assert_eq!(
        reaction.result,
        enginer_workflow::reaction::ReactionResult::Skipped
    );
}

#[test]
fn reaction_instance_pending_approval() {
    let trigger = ReactionTrigger {
        kind: ReactionTriggerKind::AssessmentThreshold,
        criterion_id: None,
        threshold: Some("90.0".to_string()),
        target_type: None,
    };
    let config = ReactionConfig::new(
        "reaction-001",
        trigger,
        ActionKind::RequestApproval,
        "dedup-001",
    );
    let mut reaction = Reaction::new(&config, "ent-001", "threshold_exceeded");
    reaction.pending_approval();
    assert_eq!(
        reaction.result,
        enginer_workflow::reaction::ReactionResult::PendingApproval
    );
}
