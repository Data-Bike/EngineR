//! Principal and group membership (docs/08 §2, ADR-010).

use enginer_domain::{GroupId, PrincipalId, TenantId};
use serde::{Deserialize, Serialize};

/// Group class: capability groups hold system rights, data groups hold data
/// rights. They never mix (ADR-010).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupClass {
    Capability,
    Data,
}

/// A principal (user or service account).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub principal_id: PrincipalId,
    pub tenant_id: TenantId,
    pub display_name: String,
    pub is_service_account: bool,
    pub status: PrincipalStatus,
}

/// Principal lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalStatus {
    Active,
    Disabled,
    Locked,
}

/// Membership of a principal in a group, with a valid interval and revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalGroupMembership {
    pub principal_id: PrincipalId,
    pub group_id: GroupId,
    pub group_class: GroupClass,
    pub valid_from: chrono::DateTime<chrono::Utc>,
    pub valid_to: Option<chrono::DateTime<chrono::Utc>>,
    pub groups_revision: i64,
}
