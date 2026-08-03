//! Declarative state machine (docs/13 §7).
//!
//! States/transitions, trigger event/manual/schedule, conditions in safe DSL,
//! required capabilities/data scope, actions, timers/SLA, assignment strategy,
//! escalation, compensation, version.

use crate::error::{WorkflowError, WorkflowResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A state in the workflow state machine.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
    /// State name (e.g. "new", "in_review", "completed").
    pub name: String,
    /// Whether this is a terminal state (no outgoing transitions).
    pub terminal: bool,
    /// Whether this is the initial state.
    pub initial: bool,
}

/// A transition guard condition (safe DSL, docs/13 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionGuard {
    /// Guard expression (evaluated by the risk engine DSL or a simple DSL).
    pub expression: String,
    /// Required capabilities (docs/08 §2).
    pub required_capabilities: Vec<String>,
    /// Required data scope.
    pub required_data_scope: Vec<String>,
}

/// A state transition (docs/13 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    /// Source state name.
    pub from: String,
    /// Target state name.
    pub to: String,
    /// Trigger: event name, "manual", or "schedule".
    pub trigger: String,
    /// Guard condition (optional).
    pub guard: Option<TransitionGuard>,
    /// Actions to execute on transition.
    pub actions: Vec<String>,
    /// Whether this transition requires approval.
    pub requires_approval: bool,
}

/// A declarative state machine definition (docs/13 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachineDefinition {
    /// Workflow definition ID.
    pub definition_id: String,
    /// Workflow version (pinned for running instances, docs/13 §7).
    pub version: String,
    /// States.
    pub states: Vec<State>,
    /// Transitions.
    pub transitions: Vec<StateTransition>,
    /// Timer/SLA definitions.
    pub timers: Vec<String>,
    /// Assignment strategy.
    pub assignment_strategy: String,
    /// Escalation rules.
    pub escalation: Vec<String>,
    /// Compensation actions.
    pub compensation: Vec<String>,
}

impl StateMachineDefinition {
    /// Get the initial state.
    pub fn initial_state(&self) -> Option<&State> {
        self.states.iter().find(|s| s.initial)
    }

    /// Get a state by name.
    pub fn get_state(&self, name: &str) -> Option<&State> {
        self.states.iter().find(|s| s.name == name)
    }

    /// Get all transitions from a given state.
    pub fn transitions_from(&self, state: &str) -> Vec<&StateTransition> {
        self.transitions
            .iter()
            .filter(|t| t.from == state)
            .collect()
    }

    /// Find a transition from one state to another.
    pub fn find_transition(&self, from: &str, to: &str) -> Option<&StateTransition> {
        self.transitions
            .iter()
            .find(|t| t.from == from && t.to == to)
    }
}

/// A running state machine instance (docs/13 §7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateMachine {
    /// Instance ID (UUID).
    pub instance_id: String,
    /// Pinned workflow definition ID.
    pub definition_id: String,
    /// Pinned workflow version.
    pub version: String,
    /// Current state name.
    pub current_state: String,
    /// Instance variables (key → value).
    pub variables: HashMap<String, serde_json::Value>,
    /// History of state transitions.
    pub history: Vec<TransitionRecord>,
    /// Whether the instance is terminated.
    pub terminated: bool,
}

/// A record of a completed transition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRecord {
    /// From state.
    pub from: String,
    /// To state.
    pub to: String,
    /// Trigger that caused the transition.
    pub trigger: String,
    /// When the transition occurred.
    pub at: chrono::DateTime<chrono::Utc>,
    /// Actor who triggered the transition.
    pub actor: String,
    /// Actions executed.
    pub actions_executed: Vec<String>,
}

impl StateMachine {
    /// Create a new instance in the initial state.
    pub fn new(definition: &StateMachineDefinition, instance_id: &str) -> WorkflowResult<Self> {
        let initial = definition
            .initial_state()
            .ok_or_else(|| WorkflowError::InvalidConfig {
                message: "no initial state defined".to_string(),
            })?;
        Ok(Self {
            instance_id: instance_id.to_string(),
            definition_id: definition.definition_id.clone(),
            version: definition.version.clone(),
            current_state: initial.name.clone(),
            variables: HashMap::new(),
            history: Vec::new(),
            terminated: false,
        })
    }

    /// Attempt a transition to a new state.
    pub fn transition(
        &mut self,
        definition: &StateMachineDefinition,
        to: &str,
        trigger: &str,
        actor: &str,
    ) -> WorkflowResult<TransitionRecord> {
        if self.terminated {
            return Err(WorkflowError::InvalidTransition {
                message: "instance is terminated".to_string(),
            });
        }
        let trans = definition
            .find_transition(&self.current_state, to)
            .ok_or_else(|| WorkflowError::InvalidTransition {
                message: format!("no transition from {} to {}", self.current_state, to),
            })?;
        if trans.trigger != trigger {
            return Err(WorkflowError::InvalidTransition {
                message: format!(
                    "trigger mismatch: expected {}, got {}",
                    trans.trigger, trigger
                ),
            });
        }
        let target_state = definition
            .get_state(to)
            .ok_or_else(|| WorkflowError::InvalidState {
                message: format!("target state {} not found", to),
            })?;
        let record = TransitionRecord {
            from: self.current_state.clone(),
            to: to.to_string(),
            trigger: trigger.to_string(),
            at: chrono::Utc::now(),
            actor: actor.to_string(),
            actions_executed: trans.actions.clone(),
        };
        self.current_state = to.to_string();
        self.history.push(record.clone());
        if target_state.terminal {
            self.terminated = true;
        }
        Ok(record)
    }

    /// Set a variable.
    pub fn set_variable(&mut self, key: &str, value: serde_json::Value) {
        self.variables.insert(key.to_string(), value);
    }

    /// Get a variable.
    pub fn get_variable(&self, key: &str) -> Option<&serde_json::Value> {
        self.variables.get(key)
    }

    /// Check if the instance is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        self.terminated
    }
}
