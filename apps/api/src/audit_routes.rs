//! Audit API routes (docs/08 §11).
//!
//! Phase 5 surface:
//! - `GET /api/v1/audit` — query audit entries with filters + cursor;
//! - `GET /api/v1/audit/checkpoint` — get latest chain checkpoint;
//! - `POST /api/v1/audit/checkpoint` — create a new checkpoint;
//! - `GET /api/v1/audit/verify` — verify chain integrity;
//! - `GET /api/v1/audit/retention` — get retention policy;
//! - `PUT /api/v1/audit/retention` — set retention policy.
//!
//! Security: `audit.view` for read, `audit.admin` for checkpoint/retention.

use crate::state::AppState;
use enginer_audit_persistence::{
    AuditRepoQuery, AuditRecord, AuditRepository, ChainCheckpoint,
    ChainVerificationResult, RetentionPolicyRecord,
};
use enginer_security_api::client::SecurityClient;
use enginer_security_api::contract::{
    ActionDecisionRequest, DecisionEffectDto, SecurityContextDto,
};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post, put, routes};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// --- DTOs ---

/// Audit query parameters (query string).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditQueryParams {
    pub kind: Option<String>,
    pub actor: Option<String>,
    pub action: Option<String>,
    pub resource_kind: Option<String>,
    pub resource_id: Option<String>,
    pub effect: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<u32>,
    pub cursor: Option<i64>,
}

// Rocket's `FromQuery` requires each field to implement `FromFormField`.
// We use individual Option<String> parameters via the query catcher macro.

/// Audit record DTO (response).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecordDto {
    pub audit_id: i64,
    pub sequence: i64,
    pub previous_hash: String,
    pub entry_hash: String,
    pub tenant_id: String,
    pub occurred_at: chrono::DateTime<chrono::Utc>,
    pub actor: String,
    pub effective_actor: Option<String>,
    pub kind: String,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: Option<String>,
    pub effect: Option<String>,
    pub reason_ids: Vec<String>,
    pub policy_revision: Option<i64>,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub query_fingerprint: Option<String>,
    pub row_count: Option<i64>,
    pub bytes: Option<i64>,
    pub client: Option<String>,
    pub service: Option<String>,
    pub metadata: serde_json::Value,
}

impl From<AuditRecord> for AuditRecordDto {
    fn from(r: AuditRecord) -> Self {
        Self {
            audit_id: r.audit_id,
            sequence: r.sequence,
            previous_hash: r.previous_hash,
            entry_hash: r.entry_hash,
            tenant_id: r.tenant_id,
            occurred_at: r.occurred_at,
            actor: r.actor,
            effective_actor: r.effective_actor,
            kind: r.kind,
            action: r.action,
            resource_kind: r.resource_kind,
            resource_id: r.resource_id,
            effect: r.effect,
            reason_ids: r.reason_ids,
            policy_revision: r.policy_revision,
            request_id: r.request_id,
            correlation_id: r.correlation_id,
            query_fingerprint: r.query_fingerprint,
            row_count: r.row_count,
            bytes: r.bytes,
            client: r.client,
            service: r.service,
            metadata: r.metadata,
        }
    }
}

/// Chain checkpoint DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCheckpointDto {
    pub checkpoint_id: i64,
    pub tenant_id: String,
    pub sequence: i64,
    pub entry_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ChainCheckpoint> for ChainCheckpointDto {
    fn from(c: ChainCheckpoint) -> Self {
        Self {
            checkpoint_id: c.checkpoint_id,
            tenant_id: c.tenant_id,
            sequence: c.sequence,
            entry_hash: c.entry_hash,
            created_at: c.created_at,
        }
    }
}

/// Chain verification result DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationDto {
    pub valid: bool,
    pub entry_count: usize,
    pub last_sequence: i64,
    pub last_hash: String,
}

impl From<ChainVerificationResult> for ChainVerificationDto {
    fn from(v: ChainVerificationResult) -> Self {
        Self {
            valid: v.valid,
            entry_count: v.entry_count,
            last_sequence: v.last_sequence,
            last_hash: v.last_hash,
        }
    }
}

/// Retention policy DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicyDto {
    pub tenant_id: String,
    pub retention_days: i32,
    pub legal_hold: bool,
    pub archive_before_delete: bool,
    pub archive_url: Option<String>,
}

impl From<RetentionPolicyRecord> for RetentionPolicyDto {
    fn from(p: RetentionPolicyRecord) -> Self {
        Self {
            tenant_id: p.tenant_id,
            retention_days: p.retention_days,
            legal_hold: p.legal_hold,
            archive_before_delete: p.archive_before_delete,
            archive_url: p.archive_url,
        }
    }
}

impl From<RetentionPolicyDto> for RetentionPolicyRecord {
    fn from(d: RetentionPolicyDto) -> Self {
        Self {
            tenant_id: d.tenant_id,
            retention_days: d.retention_days,
            legal_hold: d.legal_hold,
            archive_before_delete: d.archive_before_delete,
            archive_url: d.archive_url,
        }
    }
}

// --- Helpers ---

fn security_context(state: &AppState, purpose: &str) -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: uuid::Uuid::nil(),
        principal_id: uuid::Uuid::nil(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: purpose.to_string(),
        locale: "en-US".to_string(),
        time: chrono::Utc::now(),
        client_service: state.config.security.internal_mtls_identity.clone(),
    }
}

fn check_capability(
    state: &AppState,
    action: &str,
    purpose: &str,
) -> Result<(), rocket::http::Status> {
    if let Some(client) = &state.security_client {
        let ctx = security_context(state, purpose);
        let req = ActionDecisionRequest {
            context: ctx,
            action: action.to_string(),
            resource_kind: "audit".to_string(),
            resource_id: None,
            parameters: serde_json::Map::new(),
        };
        let dec = client
            .action_decision(req)
            .map_err(|_| rocket::http::Status::ServiceUnavailable)?;
        if !matches!(
            dec.effect,
            DecisionEffectDto::Allow | DecisionEffectDto::AllowWithObligations
        ) {
            return Err(rocket::http::Status::Forbidden);
        }
    }
    Ok(())
}

/// Get the audit repository from state (if wired).
fn audit_repo(_state: &AppState) -> Option<Arc<dyn AuditRepository>> {
    // In full deployment, this returns a PgAuditRepository wired with
    // the DB executor. For now, returns None (endpoints return 503).
    None
}

fn tenant_id(_state: &AppState) -> String {
    // In full deployment, extracted from security context / RLS setting.
    // For now, uses a placeholder.
    "00000000-0000-0000-0000-000000000000".to_string()
}

// --- Routes ---

/// GET /audit — query audit entries with filters + cursor.
#[get("/audit?<kind>&<actor>&<action>&<resource_kind>&<resource_id>&<effect>&<from>&<to>&<limit>&<cursor>")]
pub async fn query_audit(
    kind: Option<String>,
    actor: Option<String>,
    action: Option<String>,
    resource_kind: Option<String>,
    resource_id: Option<String>,
    effect: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<u32>,
    cursor: Option<i64>,
    state: &State<AppState>,
) -> Result<Json<Vec<AuditRecordDto>>, rocket::http::Status> {
    check_capability(state, "audit.view", "audit_query")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let tid = tenant_id(state);

    let query = AuditRepoQuery {
        tenant_id: tid,
        kind,
        actor,
        action,
        resource_kind,
        resource_id,
        effect,
        from: from.as_deref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&chrono::Utc))
        }),
        to: to.as_deref().and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&chrono::Utc))
        }),
        limit,
        cursor,
    };

    let records = repo
        .query(&query)
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    let dtos = records.into_iter().map(AuditRecordDto::from).collect();
    Ok(Json(dtos))
}

/// GET /audit/checkpoint — get latest chain checkpoint.
#[get("/audit/checkpoint")]
pub async fn latest_checkpoint(
    state: &State<AppState>,
) -> Result<Json<Option<ChainCheckpointDto>>, rocket::http::Status> {
    check_capability(state, "audit.view", "audit_checkpoint")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let tid = tenant_id(state);
    let cp = repo
        .latest_checkpoint(&tid)
        .map_err(|_| rocket::http::Status::InternalServerError)?
        .map(ChainCheckpointDto::from);
    Ok(Json(cp))
}

/// POST /audit/checkpoint — create a new checkpoint.
#[post("/audit/checkpoint")]
pub async fn create_checkpoint(
    state: &State<AppState>,
) -> Result<Json<ChainCheckpointDto>, rocket::http::Status> {
    check_capability(state, "audit.admin", "audit_checkpoint_create")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let tid = tenant_id(state);
    let cp = repo
        .checkpoint(&tid)
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    Ok(Json(ChainCheckpointDto::from(cp)))
}

/// GET /audit/verify — verify chain integrity.
#[get("/audit/verify")]
pub async fn verify_chain(
    state: &State<AppState>,
) -> Result<Json<ChainVerificationDto>, rocket::http::Status> {
    check_capability(state, "audit.view", "audit_verify")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let tid = tenant_id(state);
    let valid = repo
        .verify_chain(&tid)
        .map_err(|_| rocket::http::Status::InternalServerError)?;

    // Build result with chain state
    let chain_state = repo
        .chain_state(&tid)
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    let result = ChainVerificationResult {
        valid,
        entry_count: chain_state.last_sequence as usize,
        last_sequence: chain_state.last_sequence,
        last_hash: chain_state.last_hash,
    };
    Ok(Json(ChainVerificationDto::from(result)))
}

/// GET /audit/retention — get retention policy.
#[get("/audit/retention")]
pub async fn get_retention(
    state: &State<AppState>,
) -> Result<Json<RetentionPolicyDto>, rocket::http::Status> {
    check_capability(state, "audit.view", "audit_retention")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let tid = tenant_id(state);
    let policy = repo
        .retention_policy(&tid)
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    Ok(Json(RetentionPolicyDto::from(policy)))
}

/// PUT /audit/retention — set retention policy.
#[put("/audit/retention", data = "<body>")]
pub async fn set_retention(
    body: Json<RetentionPolicyDto>,
    state: &State<AppState>,
) -> Result<Json<RetentionPolicyDto>, rocket::http::Status> {
    check_capability(state, "audit.admin", "audit_retention_set")?;
    let repo = audit_repo(state).ok_or(rocket::http::Status::ServiceUnavailable)?;
    let policy = RetentionPolicyRecord::from(body.into_inner());
    repo.set_retention_policy(&policy)
        .map_err(|_| rocket::http::Status::InternalServerError)?;
    Ok(Json(RetentionPolicyDto::from(policy)))
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        query_audit,
        latest_checkpoint,
        create_checkpoint,
        verify_chain,
        get_retention,
        set_retention,
    ]
}
