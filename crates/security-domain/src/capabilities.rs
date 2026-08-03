//! System capabilities (docs/08 §2.1).
//!
//! Capability groups combine only system rights; data groups combine only data
//! rights (ADR-010). The two classes never mix.

use serde::{Deserialize, Serialize};

/// A normative system capability action. Stable string form is the wire key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityAction {
    LoginUseUi,
    EntityCreate,
    EntityEdit,
    EntityRetire,
    EntityMerge,
    QueryAdvanced,
    QueryAggregate,
    CheckRun,
    MonitorManage,
    ReportRun,
    ReportExport,
    BulkImport,
    BulkExport,
    BulkMutate,
    SchemaDraft,
    SchemaPublish,
    SecurityManage,
    SecuritySimulate,
    ConnectorManage,
    AuditRead,
    AdminPagesAccess,
}

impl CapabilityAction {
    /// Stable wire string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginUseUi => "login.use_ui",
            Self::EntityCreate => "entity.create",
            Self::EntityEdit => "entity.edit",
            Self::EntityRetire => "entity.retire",
            Self::EntityMerge => "entity.merge",
            Self::QueryAdvanced => "query.advanced",
            Self::QueryAggregate => "query.aggregate",
            Self::CheckRun => "check.run",
            Self::MonitorManage => "monitor.manage",
            Self::ReportRun => "report.run",
            Self::ReportExport => "report.export",
            Self::BulkImport => "bulk.import",
            Self::BulkExport => "bulk.export",
            Self::BulkMutate => "bulk.mutate",
            Self::SchemaDraft => "schema.draft",
            Self::SchemaPublish => "schema.publish",
            Self::SecurityManage => "security.manage",
            Self::SecuritySimulate => "security.simulate",
            Self::ConnectorManage => "connector.manage",
            Self::AuditRead => "audit.read",
            Self::AdminPagesAccess => "admin.pages.access",
        }
    }
}

/// A capability grant: an action plus optional resource qualifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub action: CapabilityAction,
    /// Optional resource qualifier (e.g. report definition id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource: Option<String>,
}
