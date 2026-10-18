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
    EntityRead,
    EntityCreate,
    EntityEdit,
    EntityRetire,
    EntityMerge,
    QueryAdvanced,
    QueryAggregate,
    CheckView,
    CheckRun,
    MonitorManage,
    ReportView,
    ReportRun,
    ReportExport,
    RiskView,
    RiskResolve,
    AssessmentView,
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
    UsersManage,
    UserRead,
    UserCreate,
    UserEdit,
    UserDelete,
}

impl CapabilityAction {
    /// Stable wire string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LoginUseUi => "login.use_ui",
            Self::EntityRead => "entity.read",
            Self::EntityCreate => "entity.create",
            Self::EntityEdit => "entity.edit",
            Self::EntityRetire => "entity.retire",
            Self::EntityMerge => "entity.merge",
            Self::QueryAdvanced => "query.advanced",
            Self::QueryAggregate => "query.aggregate",
            Self::CheckView => "check.view",
            Self::CheckRun => "check.run",
            Self::MonitorManage => "monitor.manage",
            Self::ReportView => "report.view",
            Self::ReportRun => "report.run",
            Self::ReportExport => "report.export",
            Self::RiskView => "risk.view",
            Self::RiskResolve => "risk.resolve",
            Self::AssessmentView => "assessment.view",
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
            Self::UsersManage => "users.manage",
            Self::UserRead => "user.read",
            Self::UserCreate => "user.create",
            Self::UserEdit => "user.edit",
            Self::UserDelete => "user.delete",
        }
    }

    /// Parse a stable wire string back into an action.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "login.use_ui" => Self::LoginUseUi,
            "entity.read" => Self::EntityRead,
            "entity.create" => Self::EntityCreate,
            "entity.edit" => Self::EntityEdit,
            "entity.retire" => Self::EntityRetire,
            "entity.merge" => Self::EntityMerge,
            "query.advanced" => Self::QueryAdvanced,
            "query.aggregate" => Self::QueryAggregate,
            "check.view" => Self::CheckView,
            "check.run" => Self::CheckRun,
            "monitor.manage" => Self::MonitorManage,
            "report.view" => Self::ReportView,
            "report.run" => Self::ReportRun,
            "report.export" => Self::ReportExport,
            "risk.view" => Self::RiskView,
            "risk.resolve" => Self::RiskResolve,
            "assessment.view" => Self::AssessmentView,
            "bulk.import" => Self::BulkImport,
            "bulk.export" => Self::BulkExport,
            "bulk.mutate" => Self::BulkMutate,
            "schema.draft" => Self::SchemaDraft,
            "schema.publish" => Self::SchemaPublish,
            "security.manage" => Self::SecurityManage,
            "security.simulate" => Self::SecuritySimulate,
            "connector.manage" => Self::ConnectorManage,
            "audit.read" => Self::AuditRead,
            "admin.pages.access" => Self::AdminPagesAccess,
            "users.manage" => Self::UsersManage,
            "user.read" => Self::UserRead,
            "user.create" => Self::UserCreate,
            "user.edit" => Self::UserEdit,
            "user.delete" => Self::UserDelete,
            _ => return None,
        })
    }

    /// All actions, for catalog/seed purposes.
    pub fn all() -> &'static [Self] {
        &[
            Self::LoginUseUi,
            Self::EntityRead,
            Self::EntityCreate,
            Self::EntityEdit,
            Self::EntityRetire,
            Self::EntityMerge,
            Self::QueryAdvanced,
            Self::QueryAggregate,
            Self::CheckView,
            Self::CheckRun,
            Self::MonitorManage,
            Self::ReportView,
            Self::ReportRun,
            Self::ReportExport,
            Self::RiskView,
            Self::RiskResolve,
            Self::AssessmentView,
            Self::BulkImport,
            Self::BulkExport,
            Self::BulkMutate,
            Self::SchemaDraft,
            Self::SchemaPublish,
            Self::SecurityManage,
            Self::SecuritySimulate,
            Self::ConnectorManage,
            Self::AuditRead,
            Self::AdminPagesAccess,
            Self::UsersManage,
            Self::UserRead,
            Self::UserCreate,
            Self::UserEdit,
            Self::UserDelete,
        ]
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
