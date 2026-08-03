//! Security clients (docs/08 §1, ADR-008).
//!
//! `LocalSecurityClient` calls the PDP crate in-process (no self-HTTP in the
//! hot path). `SecurityClient` is the trait both local and remote clients
//! implement, so the transport can be swapped without touching use cases.

use crate::contract::{
    ActionDecisionRequest, ActionDecisionResponse, QueryDecisionRequest, QueryDecisionResponse,
};
use enginer_domain::{PrincipalId, TenantId};
use enginer_security_pdp::{AuthorizationRequest, AuthorizationResource, PdpEngine, PdpSnapshot};
use enginer_security_domain::CapabilityAction;
use std::sync::Arc;

/// Error from a security client (fail closed).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum SecurityClientError {
    #[error("unavailable: {message}")]
    Unavailable { message: String },
    #[error("context_invalid: {message}")]
    ContextInvalid { message: String },
}

/// Snapshot provider: loads the policy + membership snapshot for a tenant +
/// principal at a given revision. Implemented by persistence in production and
/// by fakes in tests.
pub trait SnapshotProvider: Send + Sync {
    fn snapshot(
        &self,
        tenant: TenantId,
        principal: PrincipalId,
        policy_revision: Option<i64>,
    ) -> Result<PdpSnapshot, SecurityClientError>;
}

/// The security client trait (local + remote share it).
pub trait SecurityClient: Send + Sync {
    fn action_decision(
        &self,
        req: ActionDecisionRequest,
    ) -> Result<ActionDecisionResponse, SecurityClientError>;

    fn query_decision(
        &self,
        req: QueryDecisionRequest,
    ) -> Result<QueryDecisionResponse, SecurityClientError>;
}

/// In-process local client (ADR-008). Calls the PDP engine directly.
pub struct LocalSecurityClient {
    snapshot: Arc<dyn SnapshotProvider>,
    engine: PdpEngine,
}

impl LocalSecurityClient {
    /// Construct with a snapshot provider.
    pub fn new(snapshot: Arc<dyn SnapshotProvider>) -> Self {
        Self {
            snapshot,
            engine: PdpEngine,
        }
    }
}

impl SecurityClient for LocalSecurityClient {
    fn action_decision(
        &self,
        req: ActionDecisionRequest,
    ) -> Result<ActionDecisionResponse, SecurityClientError> {
        let tenant = TenantId(req.context.tenant_id);
        let principal = PrincipalId(req.context.principal_id);
        let snap = self.snapshot.snapshot(tenant, principal, None)?;

        let action = parse_capability(&req.action)
            .ok_or_else(|| SecurityClientError::ContextInvalid {
                message: format!("unknown capability: {}", req.action),
            })?;
        let pdp_req = AuthorizationRequest {
            tenant_id: tenant,
            principal_id: principal,
            auth_strength: req.context.auth_strength.clone(),
            purpose: req.context.purpose.clone(),
            time: req.context.time,
            resource: AuthorizationResource {
                action,
                resource_id: req.resource_id,
                root_type_ids: Vec::new(),
                selected_predicate_ids: Vec::new(),
            },
        };
        let dec = self
            .engine
            .evaluate_action(&snap, &pdp_req)
            .map_err(|e| SecurityClientError::ContextInvalid {
                message: format!("{e:?}"),
            })?;

        Ok(ActionDecisionResponse {
            decision_id: dec.decision_id.raw(),
            effect: dec.effect.into(),
            policy_revision: dec.policy_revision,
            obligations: serde_json::to_value(&dec.obligations).unwrap_or(serde_json::Value::Null),
            expires_at: dec.expires_at,
        })
    }

    fn query_decision(
        &self,
        req: QueryDecisionRequest,
    ) -> Result<QueryDecisionResponse, SecurityClientError> {
        let tenant = TenantId(req.context.tenant_id);
        let principal = PrincipalId(req.context.principal_id);
        let snap = self.snapshot.snapshot(tenant, principal, None)?;
        // Query decision reuses the action path with the query.aggregate /
        // query.advanced capability depending on operations; here we use
        // query.advanced as the coarse capability gate.
        let action = if req.operations.iter().any(|o| o == "aggregate") {
            CapabilityAction::QueryAggregate
        } else {
            CapabilityAction::QueryAdvanced
        };
        let pdp_req = AuthorizationRequest {
            tenant_id: tenant,
            principal_id: principal,
            auth_strength: req.context.auth_strength.clone(),
            purpose: req.context.purpose.clone(),
            time: req.context.time,
            resource: AuthorizationResource {
                action,
                resource_id: None,
                root_type_ids: req.root_type_ids.clone(),
                selected_predicate_ids: req.selected_predicate_ids.clone(),
            },
        };
        let dec = self
            .engine
            .evaluate_action(&snap, &pdp_req)
            .map_err(|e| SecurityClientError::ContextInvalid {
                message: format!("{e:?}"),
            })?;
        Ok(QueryDecisionResponse {
            decision_id: dec.decision_id.raw(),
            effect: dec.effect.into(),
            policy_revision: dec.policy_revision,
            obligations: serde_json::to_value(&dec.obligations).unwrap_or(serde_json::Value::Null),
            expires_at: dec.expires_at,
        })
    }
}

fn parse_capability(s: &str) -> Option<CapabilityAction> {
    use CapabilityAction::*;
    Some(match s {
        "login.use_ui" => LoginUseUi,
        "entity.create" => EntityCreate,
        "entity.edit" => EntityEdit,
        "entity.retire" => EntityRetire,
        "entity.merge" => EntityMerge,
        "query.advanced" => QueryAdvanced,
        "query.aggregate" => QueryAggregate,
        "check.run" => CheckRun,
        "monitor.manage" => MonitorManage,
        "report.run" => ReportRun,
        "report.export" => ReportExport,
        "bulk.import" => BulkImport,
        "bulk.export" => BulkExport,
        "bulk.mutate" => BulkMutate,
        "schema.draft" => SchemaDraft,
        "schema.publish" => SchemaPublish,
        "security.manage" => SecurityManage,
        "security.simulate" => SecuritySimulate,
        "connector.manage" => ConnectorManage,
        "audit.read" => AuditRead,
        "admin.pages.access" => AdminPagesAccess,
        _ => return None,
    })
}
