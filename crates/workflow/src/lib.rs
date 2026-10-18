//! Workflow: state machine, tasks, timers, reactions (docs/13 §4-12).
//!
//! Declarative state machine with transitions, trigger events, conditions,
//! actions, timers/SLA, assignment, escalation, compensation (docs/13 §7).
//!
//! Job state machine: queued → leased → running → waiting_external/
//! waiting_user → succeeded/partially_succeeded/failed/cancelled
//! (docs/13 §4).

#![forbid(unsafe_code)]

pub mod error;
pub mod job;
pub mod reaction;
pub mod state_machine;
pub mod task;
pub mod timer;

pub use error::{WorkflowError, WorkflowResult};
pub use job::{Job, JobState, JobStatus};
pub use reaction::{ActionKind, Reaction, ReactionConfig, ReactionTrigger, ReactionTriggerKind};
pub use state_machine::{
    State, StateMachine, StateMachineDefinition, StateTransition, TransitionGuard,
};
pub use task::{AssignmentStrategy, Task, TaskPriority, TaskStatus};
pub use timer::{EscalationTimer, MisfirePolicy, Schedule, ScheduleConfig, TimerKind};
