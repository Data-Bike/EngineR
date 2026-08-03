//! Integration: end-to-end check flow orchestrator (docs/13 §8, docs/14 §4-12).
//!
//! Wires together all Phase 5 crates into a single orchestration pipeline:
//! 1. Check request (docs/14 §4) → connector fetch (docs/14 §5-6)
//! 2. Mapping → provenance statements (docs/14 §6)
//! 3. Risk engine: criteria evaluation → factors → score → assessment
//!    (docs/14 §8-10)
//! 4. Decision: approve/reject/escalate (docs/14 §10)
//! 5. Workflow: start instance → create tasks → transition (docs/13 §7-8)
//! 6. Reactions: threshold → task/alert/report (docs/13 §10, docs/14 §12)

#![forbid(unsafe_code)]

pub mod check_flow;
pub mod error;
pub mod ports;

pub use check_flow::{
    CheckFlowContext, CheckFlowInput, CheckFlowOrchestrator, CheckFlowResult, CheckFlowState,
};
pub use error::{IntegrationError, IntegrationResult};
pub use ports::{
    AssessmentPort, AuditPort, ConnectorPort, CriterionPort, DecisionPort, FactorPort,
    TaskPort, WorkflowPort,
};
pub use enginer_workflow::{StateMachine, Task};
