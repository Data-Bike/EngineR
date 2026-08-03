//! Capabilities and capability groups (docs/08 §2).
//!
//! Two classes of rights:
//! - System capabilities: access to functions (docs/08 §2.1)
//! - Data permissions: access to data (docs/08 §2.2)
//!
//! Capability groups combine only system rights. Data groups combine only
//! data rights. A user can be a member of multiple groups of both classes.

use serde::{Deserialize, Serialize};

/// Capability kind: system or data (docs/08 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityKind {
    /// System capability: access to a function (docs/08 §2.1).
    System,
    /// Data permission: access to data (docs/08 §2.2).
    Data,
}

/// A capability (system function access, docs/08 §2.1).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Capability {
    /// Capability name (e.g. "entity.create", "report.run").
    pub name: String,
    /// Capability kind.
    pub kind: CapabilityKind,
    /// Human-readable description key (i18n).
    pub description_key: String,
    /// Whether this capability requires step-up MFA (docs/08 §9).
    pub requires_step_up: bool,
    /// Whether this capability is high-risk (bypasses policy cache, docs/08 §10).
    pub high_risk: bool,
}

/// Capability group kind (docs/08 §2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityGroupKind {
    /// System group: combines only system capabilities (docs/08 §2.1).
    System,
    /// Data group: combines only data permissions (docs/08 §2.2).
    Data,
}

/// A capability group (docs/08 §2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityGroup {
    /// Group ID.
    pub group_id: String,
    /// Group name.
    pub name: String,
    /// Group kind (system or data).
    pub kind: CapabilityGroupKind,
    /// Capabilities in this group.
    pub capabilities: Vec<String>,
    /// Description key (i18n).
    pub description_key: String,
    /// Owner (team or user responsible).
    pub owner: String,
}

impl CapabilityGroup {
    /// Check if the group has a capability.
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities.contains(&capability.to_string())
    }

    /// Add a capability to the group.
    pub fn add_capability(&mut self, capability: &str) {
        if !self.has_capability(capability) {
            self.capabilities.push(capability.to_string());
        }
    }

    /// Remove a capability from the group.
    pub fn remove_capability(&mut self, capability: &str) {
        self.capabilities.retain(|c| c != capability);
    }
}

/// Standard system capabilities (docs/08 §2.1).
pub fn standard_system_capabilities() -> Vec<Capability> {
    vec![
        Capability {
            name: "login.use_ui".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.login.use_ui".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "entity.create".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.entity.create".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "entity.edit".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.entity.edit".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "entity.retire".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.entity.retire".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "entity.merge".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.entity.merge".to_string(),
            requires_step_up: false,
            high_risk: true,
        },
        Capability {
            name: "query.advanced".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.query.advanced".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "query.aggregate".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.query.aggregate".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "check.run".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.check.run".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "check.monitor.manage".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.check.monitor.manage".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "report.run".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.report.run".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "report.export".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.report.export".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
        Capability {
            name: "bulk.import".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.bulk.import".to_string(),
            requires_step_up: false,
            high_risk: true,
        },
        Capability {
            name: "bulk.export".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.bulk.export".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
        Capability {
            name: "bulk.mutate".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.bulk.mutate".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
        Capability {
            name: "schema.draft".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.schema.draft".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "schema.publish".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.schema.publish".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
        Capability {
            name: "security.manage".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.security.manage".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
        Capability {
            name: "security.simulate".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.security.simulate".to_string(),
            requires_step_up: true,
            high_risk: false,
        },
        Capability {
            name: "connector.manage".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.connector.manage".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "audit.read".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.audit.read".to_string(),
            requires_step_up: false,
            high_risk: false,
        },
        Capability {
            name: "admin.pages.access".to_string(),
            kind: CapabilityKind::System,
            description_key: "capability.admin.pages.access".to_string(),
            requires_step_up: true,
            high_risk: true,
        },
    ]
}
