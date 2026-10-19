//! PostgreSQL-backed audit + permission-group endpoints (docs/08 §11, §15).
//!
//! - `GET  /audit` — query `audit.event` with filters and cursor pagination.
//! - `GET|POST /audit/checkpoint` — chain checkpoints.
//! - `POST /audit/verify` — verify chain integrity (sequence + hash linkage).
//! - `GET|PUT /audit/retention` — per-tenant retention policy.
//! - `*/permissions/groups` — `sec.group` + memberships + policy rules.
//!
//! `record_audit_event` appends a tamper-evident row (login, denied requests,
//! policy changes). Rows are insert-only; verification recomputes the hash
//! chain end-to-end.

use crate::authn::AuthContext;
use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use crate::AppState;
use enginer_audit_persistence::chain::ChainHasher;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use enginer_persistence::RlsContext;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, put, routes, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::postgres::PgPool;
use sqlx::{Postgres, Row, Transaction};
use std::sync::Arc;

fn db(e: sqlx::Error) -> DbError {
    DbError::Other {
        message: e.to_string(),
    }
}

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, Status> {
    state.db_executor.clone().ok_or(Status::ServiceUnavailable)
}

pub(crate) async fn audit_tx<'e>(
    pool: &'e PgPool,
    ctx: &AuthContext,
    purpose: &str,
) -> Result<Transaction<'e, Postgres>, DbError> {
    let mut tx = pool.begin().await.map_err(db)?;
    RlsContext {
        tenant_id: enginer_domain::TenantId(ctx.tenant_id),
        principal_id: enginer_domain::PrincipalId(ctx.principal_id),
        purpose: purpose.to_string(),
        policy_revision: None,
    }
    .install_tx(&mut tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(tx)
}

// ---------------------------------------------------------------------------
// Audit event writer (tamper-evident chain, docs/08 §11)
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
async fn insert_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    kind: &str,
    action: &str,
    resource_kind: &str,
    resource_id: Option<uuid::Uuid>,
    actor: &str,
    principal_id: Option<uuid::Uuid>,
    session_id: Option<uuid::Uuid>,
    effect: Option<&str>,
    outcome: &str,
    client: Option<&str>,
    service: &str,
    metadata: Value,
) -> Result<(), DbError> {
    // Serialize chain assignment per tenant.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext($1::text))")
        .bind(tenant)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    let last: Option<(i64, Option<String>)> = sqlx::query_as(
        "SELECT sequence, entry_hash FROM audit.event
         WHERE tenant_id=$1 AND sequence IS NOT NULL
         ORDER BY sequence DESC LIMIT 1",
    )
    .bind(tenant)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    let sequence = last.as_ref().map(|(s, _)| s + 1).unwrap_or(1);
    let prev_hash = last.and_then(|(_, h)| h).unwrap_or_else(|| "0".repeat(64));
    // Truncate to microseconds — timestamptz precision — so the stored row
    // and the hash input agree (verify recomputes from stored fields).
    let occurred_at = chrono::DateTime::<chrono::Utc>::from_timestamp_micros(
        chrono::Utc::now().timestamp_micros(),
    )
    .unwrap_or_else(chrono::Utc::now);
    let entry_hash = ChainHasher::compute_hash(
        sequence,
        &prev_hash,
        occurred_at,
        &tenant.to_string(),
        actor,
        action,
        kind,
        resource_kind,
        resource_id.map(|u| u.to_string()).as_deref(),
        effect,
    )
    .map_err(|e| DbError::Other {
        message: format!("hash: {e}"),
    })?;
    sqlx::query(
        "INSERT INTO audit.event
         (tenant_id, sequence, previous_hash, entry_hash, kind, action,
          resource_kind, resource_id, principal_id, session_id, effect,
          outcome, occurred_at, client, service, metadata)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)",
    )
    .bind(tenant)
    .bind(sequence)
    .bind(&prev_hash)
    .bind(&entry_hash)
    .bind(kind)
    .bind(action)
    .bind(resource_kind)
    .bind(resource_id)
    .bind(principal_id)
    .bind(session_id)
    .bind(effect)
    .bind(outcome)
    .bind(occurred_at)
    .bind(client)
    .bind(service)
    .bind(metadata)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    Ok(())
}

/// Append an audit event. Best-effort: failures are logged, never returned.
/// `authn` may be None (pre-auth events); RLS is installed with tenant only.
pub async fn record_audit_event(
    exec: Arc<AsyncStdDbExecutor>,
    tenant: uuid::Uuid,
    principal_id: Option<uuid::Uuid>,
    session_id: Option<uuid::Uuid>,
    kind: impl Into<String>,
    action: impl Into<String>,
    resource_kind: impl Into<String>,
    resource_id: Option<uuid::Uuid>,
    effect: Option<impl Into<String>>,
    outcome: impl Into<String>,
    client: Option<String>,
    metadata: Value,
) {
    let kind = kind.into();
    let action = action.into();
    let resource_kind = resource_kind.into();
    let effect = effect.map(Into::into);
    let outcome = outcome.into();
    let result = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let mut tx = pool.begin().await.map_err(db)?;
            // RLS: the actor may differ from the audited principal (e.g.
            // pre-auth failures); install context with tenant only.
            sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
                .bind(tenant.to_string())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            sqlx::query("SELECT set_config('app.principal_id', $1, true)")
                .bind(principal_id.unwrap_or(uuid::Uuid::nil()).to_string())
                .execute(&mut *tx)
                .await
                .map_err(db)?;
            insert_event(
                &mut tx,
                tenant,
                &kind,
                &action,
                &resource_kind,
                resource_id,
                &principal_id
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "system".to_string()),
                principal_id,
                session_id,
                effect.as_deref(),
                &outcome,
                client.as_deref(),
                "enginer-api",
                metadata,
            )
            .await?;
            tx.commit().await.map_err(db)?;
            Ok(Value::Null)
        })
        .await;
    if let Err(e) = result {
        tracing::warn!(error = %e, "audit event write failed");
    }
}

// ---------------------------------------------------------------------------
// Audit query
// ---------------------------------------------------------------------------

fn row_to_record(r: &sqlx::postgres::PgRow) -> Value {
    let metadata: Value = r.try_get("metadata").unwrap_or(Value::Null);
    let occurred: Option<chrono::DateTime<chrono::Utc>> = r.try_get("occurred_at").ok();
    let principal: Option<uuid::Uuid> = r.try_get("principal_id").ok().flatten();
    let actor_login: Option<String> = r.try_get("actor_login").ok().flatten();
    let resource: Option<uuid::Uuid> = r.try_get("resource_id").ok().flatten();
    let request: Option<uuid::Uuid> = r.try_get("request_id").ok().flatten();
    let correlation: Option<uuid::Uuid> = r.try_get("correlation_id").ok().flatten();
    let reasons: Option<Vec<uuid::Uuid>> = r.try_get("reason_ids").ok().flatten();
    json!({
        "audit_id": r.try_get::<i64, _>("audit_id").unwrap_or_default(),
        "sequence": r.try_get::<Option<i64>, _>("sequence").ok().flatten().unwrap_or_default(),
        "occurred_at": occurred.map(|t| t.to_rfc3339()).unwrap_or_default(),
        "kind": r.try_get::<String, _>("kind").unwrap_or_default(),
        "actor": actor_login.or_else(|| principal.map(|p| p.to_string())).unwrap_or_default(),
        "effective_actor": r.try_get::<Option<uuid::Uuid>, _>("effective_actor").ok().flatten().map(|u| u.to_string()),
        "action": r.try_get::<String, _>("action").unwrap_or_default(),
        "resource_kind": r.try_get::<String, _>("resource_kind").unwrap_or_default(),
        "resource_id": resource.map(|u| u.to_string()),
        "effect": r.try_get::<Option<String>, _>("effect").ok().flatten(),
        "reason_ids": reasons.unwrap_or_default().iter().map(|u| u.to_string()).collect::<Vec<_>>(),
        "entry_hash": r.try_get::<Option<String>, _>("entry_hash").ok().flatten().unwrap_or_default(),
        "previous_hash": r.try_get::<Option<String>, _>("previous_hash").ok().flatten().unwrap_or_default(),
        "request_id": request.map(|u| u.to_string()),
        "correlation_id": correlation.map(|u| u.to_string()),
        "policy_revision": r.try_get::<Option<i64>, _>("policy_revision").ok().flatten(),
        "query_fingerprint": r.try_get::<Option<String>, _>("query_fingerprint").ok().flatten(),
        "row_count": r.try_get::<Option<i64>, _>("row_count").ok().flatten(),
        "bytes": r.try_get::<Option<i64>, _>("bytes").ok().flatten(),
        "client": r.try_get::<Option<String>, _>("client").ok().flatten(),
        "service": r.try_get::<Option<String>, _>("service").ok().flatten(),
        "metadata": metadata,
    })
}

#[get("/audit?<kind>&<actor>&<action>&<resource_kind>&<resource_id>&<effect>&<from>&<to>&<limit>&<cursor>")]
pub async fn query_audit(
    cap: CapabilityGuard,
    state: &State<AppState>,
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
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.read").await?;
            let limit = limit.unwrap_or(100).clamp(1, 500) as i64;
            let actor_uuid = actor.as_deref().and_then(|s| s.parse::<uuid::Uuid>().ok());
            let actor_login = actor.filter(|s| s.parse::<uuid::Uuid>().is_err());
            let resource_uuid = resource_id
                .as_deref()
                .and_then(|s| s.parse::<uuid::Uuid>().ok());
            let from_ts = from
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
            let to_ts = to
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
            let rows = sqlx::query(
                "SELECT e.audit_id, e.sequence, e.occurred_at, e.kind, e.principal_id,
                        e.effective_actor, e.action, e.resource_kind, e.resource_id,
                        e.effect, e.reason_ids, e.entry_hash, e.previous_hash,
                        e.request_id, e.correlation_id, e.policy_revision,
                        e.query_fingerprint, e.row_count, e.bytes, e.client,
                        e.service, e.metadata,
                        c.login AS actor_login
                 FROM audit.event e
                 LEFT JOIN sec.principal_credential c
                     ON c.principal_id = e.principal_id
                 WHERE e.tenant_id = $1
                   AND ($2::text IS NULL OR e.kind = $2)
                   AND ($3::uuid IS NULL OR e.principal_id = $3)
                   AND ($4::text IS NULL OR c.login = $4)
                   AND ($5::text IS NULL OR e.action = $5)
                   AND ($6::text IS NULL OR e.resource_kind = $6)
                   AND ($7::uuid IS NULL OR e.resource_id = $7)
                   AND ($8::text IS NULL OR e.effect = $8)
                   AND ($9::timestamptz IS NULL OR e.occurred_at >= $9)
                   AND ($10::timestamptz IS NULL OR e.occurred_at <= $10)
                   AND ($11::bigint IS NULL OR e.sequence < $11)
                 ORDER BY e.sequence DESC NULLS LAST
                 LIMIT $12",
            )
            .bind(ctx.tenant_id)
            .bind(kind)
            .bind(actor_uuid)
            .bind(actor_login)
            .bind(action)
            .bind(resource_kind)
            .bind(resource_uuid)
            .bind(effect)
            .bind(from_ts.map(|t| t.with_timezone(&chrono::Utc)))
            .bind(to_ts.map(|t| t.with_timezone(&chrono::Utc)))
            .bind(cursor)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db)?;
            let out: Vec<Value> = rows.iter().map(row_to_record).collect();
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "audit query failed");
            Status::InternalServerError
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Checkpoints + verification
// ---------------------------------------------------------------------------

#[get("/audit/checkpoint")]
pub async fn latest_checkpoint(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.read").await?;
            let row: Option<(i64, i64, String, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
                "SELECT checkpoint_id, sequence, entry_hash, created_at
                 FROM audit.chain_checkpoint WHERE tenant_id=$1
                 ORDER BY checkpoint_id DESC LIMIT 1",
            )
            .bind(ctx.tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((cid, seq, hash, created)) = row else {
                return Err(DbError::Other {
                    message: "HTTP404:checkpoint".into(),
                });
            };
            Ok(json!({
                "checkpoint_id": cid, "sequence": seq,
                "entry_hash": hash, "created_at": created.to_rfc3339(),
            }))
        })
        .await
        .map_err(|e| match e {
            DbError::Other { message } if message == "HTTP404:checkpoint" => Status::NotFound,
            _ => Status::InternalServerError,
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/audit/checkpoint")]
pub async fn create_checkpoint(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.checkpoint").await?;
            let last: Option<(i64, String)> = sqlx::query_as(
                "SELECT sequence, entry_hash FROM audit.event
                 WHERE tenant_id=$1 AND sequence IS NOT NULL
                 ORDER BY sequence DESC LIMIT 1",
            )
            .bind(ctx.tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let Some((seq, hash)) = last else {
                return Err(DbError::Other {
                    message: "HTTP409:checkpoint".into(),
                });
            };
            let row: (i64, chrono::DateTime<chrono::Utc>) = sqlx::query_as(
                "INSERT INTO audit.chain_checkpoint (tenant_id, sequence, entry_hash)
                 VALUES ($1,$2,$3) RETURNING checkpoint_id, created_at",
            )
            .bind(ctx.tenant_id)
            .bind(seq)
            .bind(&hash)
            .fetch_one(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            Ok(json!({
                "checkpoint_id": row.0, "sequence": seq,
                "entry_hash": hash, "created_at": row.1.to_rfc3339(),
            }))
        })
        .await
        .map_err(|e| match e {
            DbError::Other { message } if message == "HTTP409:checkpoint" => Status::Conflict,
            _ => Status::InternalServerError,
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/audit/verify")]
pub async fn verify_chain(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.verify").await?;
            let mut rows = sqlx::query(
                "SELECT e.sequence, e.previous_hash, e.entry_hash, e.occurred_at,
                        e.tenant_id, e.kind, e.action, e.resource_kind, e.resource_id,
                        e.effect, e.principal_id
                 FROM audit.event e
                 WHERE e.tenant_id=$1 AND e.sequence IS NOT NULL
                 ORDER BY e.sequence ASC",
            )
            .bind(ctx.tenant_id)
            .fetch(&mut *tx);
            let mut valid = true;
            let mut entry_count = 0i64;
            let mut prev_hash = "0".repeat(64);
            let mut prev_seq = 0i64;
            use rocket::futures::StreamExt as _;
            use sqlx::Row as _;
            while let Some(r) = rows.next().await {
                let r = r.map_err(db)?;
                let seq: i64 = r.try_get("sequence").unwrap_or_default();
                let stored_prev: String = r.try_get("previous_hash").unwrap_or_default();
                let stored_hash: String = r.try_get("entry_hash").unwrap_or_default();
                if seq != prev_seq + 1 || stored_prev != prev_hash {
                    valid = false;
                    break;
                }
                // Recompute the entry hash from stored fields.
                let tenant: uuid::Uuid = r.try_get("tenant_id").unwrap_or_default();
                let occurred: chrono::DateTime<chrono::Utc> =
                    r.try_get("occurred_at").unwrap_or_default();
                let principal: Option<uuid::Uuid> = r.try_get("principal_id").ok().flatten();
                let actor = principal
                    .map(|p| p.to_string())
                    .unwrap_or_else(|| "system".to_string());
                let resource: Option<uuid::Uuid> = r.try_get("resource_id").ok().flatten();
                let recomputed = ChainHasher::compute_hash(
                    seq,
                    &stored_prev,
                    occurred,
                    &tenant.to_string(),
                    &actor,
                    r.try_get("action").unwrap_or_default(),
                    r.try_get("kind").unwrap_or_default(),
                    r.try_get("resource_kind").unwrap_or_default(),
                    resource.map(|u| u.to_string()).as_deref(),
                    r.try_get::<Option<String>, _>("effect")
                        .ok()
                        .flatten()
                        .as_deref(),
                );
                match recomputed {
                    Ok(h) if h == stored_hash => {}
                    _ => {
                        valid = false;
                        break;
                    }
                }
                prev_hash = stored_hash;
                prev_seq = seq;
                entry_count += 1;
            }
            Ok(json!({
                "valid": valid,
                "entry_count": entry_count,
                "last_sequence": prev_seq,
            }))
        })
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "audit verify failed");
            Status::InternalServerError
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Retention policy
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RetentionPolicyBody {
    pub retention_days: i32,
    #[serde(default)]
    pub legal_hold: bool,
    #[serde(default = "default_true")]
    pub archive_before_delete: bool,
    #[serde(default)]
    pub archive_url: Option<String>,
}

fn default_true() -> bool {
    true
}

#[get("/audit/retention")]
pub async fn get_retention(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.read").await?;
            let row: Option<(i32, bool, bool, Option<String>)> = sqlx::query_as(
                "SELECT retention_days, legal_hold, archive_before_delete, archive_url
                 FROM audit.retention_policy WHERE tenant_id=$1",
            )
            .bind(ctx.tenant_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            let (days, hold, archive, url) = row.unwrap_or((365, false, true, None));
            Ok(json!({
                "retention_days": days, "legal_hold": hold,
                "archive_before_delete": archive, "archive_url": url,
            }))
        })
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[put("/audit/retention", data = "<policy>")]
pub async fn put_retention(
    cap: CapabilityGuard,
    state: &State<AppState>,
    policy: Json<RetentionPolicyBody>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let policy = policy.into_inner();
    if !(1..=36500).contains(&policy.retention_days) {
        return Err(Status::UnprocessableEntity);
    }
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "audit.admin").await?;
            sqlx::query(
                "INSERT INTO audit.retention_policy
                 (tenant_id, retention_days, legal_hold, archive_before_delete, archive_url, updated_at)
                 VALUES ($1,$2,$3,$4,$5,now())
                 ON CONFLICT (tenant_id) DO UPDATE
                 SET retention_days=EXCLUDED.retention_days,
                     legal_hold=EXCLUDED.legal_hold,
                     archive_before_delete=EXCLUDED.archive_before_delete,
                     archive_url=EXCLUDED.archive_url,
                     updated_at=now()",
            )
            .bind(ctx.tenant_id)
            .bind(policy.retention_days)
            .bind(policy.legal_hold)
            .bind(policy.archive_before_delete)
            .bind(&policy.archive_url)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            tx.commit().await.map_err(db)?;
            Ok(serde_json::to_value(&policy).unwrap())
        })
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Permission groups (sec.group + memberships + policy_rule)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRuleRecord {
    pub effect: String,
    /// `all|type|field|entity|relation|relation_instance|url|capability`
    pub resource_type: String,
    /// Object-type alias (for `type`/`field` scopes).
    #[serde(default)]
    pub type_alias: String,
    #[serde(default)]
    pub predicate: String,
    #[serde(default)]
    pub direction: String,
    #[serde(default)]
    pub field: String,
    /// Concrete entity public_id (for `entity` scope).
    #[serde(default)]
    pub entity: String,
    /// Concrete relation statement id (for `relation_instance` scope).
    #[serde(default)]
    pub relation: String,
    /// URL path prefix (for `url` scope).
    #[serde(default)]
    pub path: String,
    /// `read|create|update|delete|*` for data rules; raw capability string for
    /// `capability` rules (e.g. `entity.read`, `admin.pages.access`).
    pub action: String,
    /// Field mask (`omit|partial-mask|null-mask`); absent/null/empty = allow.
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub mask: String,
}

fn deserialize_null_default<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
pub struct PermissionGroupInput {
    pub name: String,
    #[serde(default)]
    pub description: String,
    /// `capability` (system-access role) or `data` (data-access role).
    #[serde(default = "default_group_class")]
    pub class: String,
    /// Parent role id for the role tree (inherits all parent rules).
    #[serde(default)]
    pub parent_id: Option<String>,
    #[serde(default)]
    pub principals: Vec<String>,
    #[serde(default)]
    pub rules: Vec<PermissionRuleRecord>,
}

fn default_group_class() -> String {
    "capability".to_string()
}

/// Map a UI-level rule (`resource_type` + op `action`) to the canonical
/// `sec.policy_rule.action` + `resource` selector.
fn canonical_rule(rule: &PermissionRuleRecord) -> Option<(String, Value)> {
    let rt = rule.resource_type.as_str();
    // URL rules carry no CRUD op — accept any action string ('access', '*',
    // whatever the UI sent); the path prefix is the selector.
    if rt == "url" {
        let mut res = serde_json::Map::new();
        res.insert("path".into(), json!(rule.path));
        return Some(("system.url".to_string(), Value::Object(res)));
    }
    let op = match rule.action.as_str() {
        "read" | "create" | "update" | "delete" | "*" => rule.action.clone(),
        // Capability rules keep the raw capability action verbatim.
        other if rt == "capability" => other.to_string(),
        _ => return None,
    };
    let mut res = serde_json::Map::new();
    let action = match rt {
        "all" => {
            if op == "*" {
                "data.*".to_string()
            } else {
                format!("data.all.{op}")
            }
        }
        "type" => {
            res.insert("type".into(), json!(rule.type_alias));
            format!("data.type.{op}")
        }
        "field" => {
            res.insert("type".into(), json!(rule.type_alias));
            res.insert("field".into(), json!(rule.field));
            format!("data.field.{op}")
        }
        "entity" => {
            res.insert("entity".into(), json!(rule.entity));
            format!("data.entity.{op}")
        }
        "relation" | "predicate" => {
            res.insert("predicate".into(), json!(rule.predicate));
            res.insert("direction".into(), json!(rule.direction));
            format!("data.relation.{op}")
        }
        "relation_instance" => {
            res.insert("relation".into(), json!(rule.relation));
            format!("data.relation_instance.{op}")
        }
        "capability" => op.clone(),
        _ => return None,
    };
    Some((action, Value::Object(res)))
}

/// Inverse of `canonical_rule`: split the stored action + resource into the
/// flat UI shape.
fn parse_rule(action: &str, resource: &Value) -> (String, String) {
    let get = |k: &str| resource.get(k).and_then(Value::as_str).unwrap_or_default();
    if action == "system.url" {
        // '*' round-trips through the UI action select (no per-op URL verbs).
        return ("url".to_string(), "*".to_string());
    }
    if action == "data.*" || action == "data.all.*" {
        return ("all".to_string(), "*".to_string());
    }
    if let Some(rest) = action.strip_prefix("data.") {
        let mut it = rest.splitn(2, '.');
        if let (Some(scope), Some(op)) = (it.next(), it.next()) {
            let rt = match scope {
                "entity_type" => "type",
                "object" => "entity",
                "predicate" => "relation",
                "statement" => "relation_instance",
                other => other,
            };
            let _ = get;
            return (rt.to_string(), op.to_string());
        }
    }
    ("capability".to_string(), action.to_string())
}

fn validate_group_input(input: &PermissionGroupInput) -> Result<(), Status> {
    // The PDP resolves special roles by `COALESCE(special, alias)`: a group
    // merely NAMED 'superadmin'/'superuser' grants its members the special
    // powers, so the aliases are reserved and cannot be created or renamed
    // into through this endpoint (the real presets are seeded, flagged via
    // `sec.group.special`, and not editable/deletable).
    let name = input.name.trim();
    if name.is_empty()
        || name.eq_ignore_ascii_case("superadmin")
        || name.eq_ignore_ascii_case("superuser")
        || !matches!(input.class.as_str(), "capability" | "data")
        || input.rules.iter().any(|rule| {
            !matches!(rule.effect.as_str(), "allow" | "deny")
                || rule.resource_type.trim().is_empty()
                || rule.action.trim().is_empty()
                || canonical_rule(rule).is_none()
        })
    {
        return Err(Status::UnprocessableEntity);
    }
    Ok(())
}

/// `parent_id` must reference an existing non-special group in the same
/// tenant. Inheriting from a special preset role would grant the child (and
/// every principal under it) superadmin/superuser powers — the flags are
/// resolved through the parent chain. A missing/cross-tenant parent is a
/// 422, not a foreign-key 500.
async fn validate_parent_group(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    parent: Option<uuid::Uuid>,
) -> Result<(), DbError> {
    let Some(pid) = parent else { return Ok(()) };
    let row: Option<(String, String)> = sqlx::query_as(
        "SELECT COALESCE(special,''), alias FROM sec.\"group\"
         WHERE tenant_id=$1 AND group_id=$2",
    )
    .bind(tenant)
    .bind(pid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db)?;
    match row {
        None => Err(DbError::Other {
            message: "HTTP422:parent group not found".into(),
        }),
        Some((special, alias))
            if !special.is_empty()
                || alias.eq_ignore_ascii_case("superadmin")
                || alias.eq_ignore_ascii_case("superuser") =>
        {
            Err(DbError::Other {
                message: "HTTP422:cannot inherit from a special role".into(),
            })
        }
        _ => Ok(()),
    }
}

async fn load_groups(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
) -> Result<Vec<Value>, DbError> {
    let rows = sqlx::query(
        "SELECT g.group_id, g.alias, g.description, g.class, g.parent_id,
                COALESCE(g.special, '') AS special_flag,
                COALESCE((SELECT jsonb_agg(DISTINCT c.login)
                 FROM sec.principal_group_membership m
                 JOIN sec.principal_credential c ON c.principal_id = m.principal_id
                 WHERE m.group_id = g.group_id AND m.tenant_id = g.tenant_id
                   AND (m.valid_to IS NULL OR m.valid_to > now())), '[]'::jsonb) AS principals,
                COALESCE((SELECT jsonb_agg(jsonb_build_object(
                     'effect', r.effect, 'action', r.action,
                     'resource', r.resource,
                     'mask', COALESCE(r.field_mask,''),
                     'reason', COALESCE(r.reason,'')))
                 FROM sec.policy_rule r
                 WHERE r.group_id = g.group_id AND r.tenant_id = g.tenant_id
                   AND (r.valid_to IS NULL OR r.valid_to > now())), '[]'::jsonb) AS rules
         FROM sec.\"group\" g WHERE g.tenant_id=$1
         ORDER BY g.alias",
    )
    .bind(tenant)
    .fetch_all(&mut **tx)
    .await
    .map_err(db)?;
    Ok(rows
        .iter()
        .map(|r| {
            let gid: uuid::Uuid = r.try_get("group_id").unwrap_or_default();
            let rules_raw = r.try_get::<Value, _>("rules").unwrap_or(json!([]));
            let rules: Vec<Value> = rules_raw
                .as_array()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|rule| {
                    let action = rule["action"].as_str().unwrap_or_default();
                    let resource = &rule["resource"];
                    let (rt, op) = parse_rule(action, resource);
                    let res = |k: &str| {
                        resource
                            .get(k)
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_string()
                    };
                    json!({
                        "effect": rule["effect"].as_str().unwrap_or("allow"),
                        "resource_type": rt,
                        "action": op,
                        "type_alias": res("type"),
                        "predicate": res("predicate"),
                        "direction": res("direction"),
                        "field": res("field"),
                        "entity": res("entity"),
                        "relation": res("relation"),
                        "path": res("path"),
                        "mask": rule["mask"].as_str().unwrap_or_default(),
                        "reason": rule["reason"].as_str().unwrap_or_default(),
                    })
                })
                .collect();
            let alias = r.try_get::<String, _>("alias").unwrap_or_default();
            let special = !r
                .try_get::<String, _>("special_flag")
                .unwrap_or_default()
                .is_empty();
            json!({
                "id": gid.to_string(),
                "name": alias,
                "description": r.try_get::<Option<String>, _>("description").ok().flatten().unwrap_or_default(),
                "class": r.try_get::<String, _>("class").unwrap_or_default(),
                "parent_id": r.try_get::<Option<uuid::Uuid>, _>("parent_id")
                    .ok().flatten().map(|u| u.to_string()),
                "special": special,
                "principals": r.try_get::<Value, _>("principals").unwrap_or(json!([])),
                "rules": rules,
            })
        })
        .collect())
}

async fn current_policy_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
) -> Result<i64, DbError> {
    let rev: Option<i64> = sqlx::query_scalar(
        "SELECT COALESCE(MAX(revision_id),1) FROM sec.policy_revision WHERE tenant_id=$1",
    )
    .bind(tenant)
    .fetch_one(&mut **tx)
    .await
    .map_err(db)?;
    Ok(rev.unwrap_or(1))
}

async fn write_group(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    group_id: uuid::Uuid,
    input: &PermissionGroupInput,
) -> Result<(), DbError> {
    // Resolve member logins up front: an unknown login must fail loudly
    // instead of silently shrinking the member list.
    let mut member_ids: Vec<uuid::Uuid> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    for login in &input.principals {
        let pid: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT principal_id FROM sec.principal_credential
             WHERE login=$1 AND tenant_id=$2",
        )
        .bind(login.trim())
        .bind(tenant)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        match pid {
            Some((pid,)) => member_ids.push(pid),
            None => unknown.push(login.trim().to_string()),
        }
    }
    if !unknown.is_empty() {
        return Err(DbError::Other {
            message: format!("HTTP422:unknown principals: {}", unknown.join(", ")),
        });
    }
    // Memberships and rules are versioned rows: close (valid_to = now) what is
    // no longer present and insert fresh rows, keeping history auditable.
    let now = chrono::Utc::now();
    sqlx::query(
        "UPDATE sec.principal_group_membership SET valid_to = $3
         WHERE tenant_id=$1 AND group_id=$2
           AND (valid_to IS NULL OR valid_to > $3)
           AND NOT (principal_id = ANY($4))",
    )
    .bind(tenant)
    .bind(group_id)
    .bind(now)
    .bind(&member_ids)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    for pid in &member_ids {
        // Application-clock valid_from (see users_pg::set_memberships):
        // PDP compares it against app-clock request time.
        sqlx::query(
            "INSERT INTO sec.principal_group_membership
             (tenant_id, principal_id, group_id, valid_from)
             SELECT $1,$2,$3,$4
             WHERE NOT EXISTS (
                 SELECT 1 FROM sec.principal_group_membership m
                 WHERE m.tenant_id=$1 AND m.principal_id=$2 AND m.group_id=$3
                   AND (m.valid_to IS NULL OR m.valid_to > $4))",
        )
        .bind(tenant)
        .bind(pid)
        .bind(group_id)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    // Replace rules at the current policy revision (soft-close, keep history).
    let revision = current_policy_revision(tx, tenant).await?;
    sqlx::query(
        "UPDATE sec.policy_rule SET valid_to = $3
         WHERE tenant_id=$1 AND group_id=$2
           AND (valid_to IS NULL OR valid_to > $3)",
    )
    .bind(tenant)
    .bind(group_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    for rule in &input.rules {
        let Some((action, resource)) = canonical_rule(rule) else {
            continue;
        };
        sqlx::query(
            // Application-clock valid_from: the PDP compares it against
            // request time (app clock); a DB default is skew-unsafe.
            "INSERT INTO sec.policy_rule
             (tenant_id, policy_revision, effect, group_id, action, resource,
              row_condition, field_mask, priority, reason, valid_from)
             VALUES ($1,$2,$3,$4,$5,$6,NULL,$7,0,$8,$9)",
        )
        .bind(tenant)
        .bind(revision)
        .bind(&rule.effect)
        .bind(group_id)
        .bind(&action)
        .bind(resource)
        .bind(if rule.mask.is_empty() {
            None
        } else {
            Some(&rule.mask)
        })
        .bind("set via /permissions/groups")
        .bind(chrono::Utc::now())
        .execute(&mut **tx)
        .await
        .map_err(db)?;
    }
    Ok(())
}

#[get("/permissions/groups")]
pub async fn list_permission_groups(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "permissions.read").await?;
            Ok(Value::Array(load_groups(&mut tx, ctx.tenant_id).await?))
        })
        .await
        .map_err(|_| Status::InternalServerError)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/permissions/groups", data = "<body>")]
pub async fn create_permission_group(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: Json<PermissionGroupInput>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let body = body.into_inner();
    validate_group_input(&body)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "permissions.admin").await?;
            let gid = uuid::Uuid::now_v7();
            let parent: Option<uuid::Uuid> = body
                .parent_id
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .and_then(|s| s.parse().ok());
            validate_parent_group(&mut tx, ctx.tenant_id, parent).await?;
            sqlx::query(
                "INSERT INTO sec.\"group\" (group_id, tenant_id, class, alias, description, parent_id)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(gid)
            .bind(ctx.tenant_id)
            .bind(&body.class)
            .bind(body.name.trim())
            .bind(&body.description)
            .bind(parent)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
            write_group(&mut tx, ctx.tenant_id, gid, &body).await?;
            tx.commit().await.map_err(db)?;
            let mut tx = audit_tx(&pool, &ctx, "permissions.read").await?;
            let groups = load_groups(&mut tx, ctx.tenant_id).await?;
            Ok(groups
                .into_iter()
                .find(|g| g["id"].as_str() == Some(&gid.to_string()))
                .unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| {
            if let DbError::Other { message } = &e {
                if let Some(detail) = message.strip_prefix("HTTP422:") {
                    tracing::warn!(detail = %detail, "permission group rejected");
                    return Status::UnprocessableEntity;
                }
            }
            tracing::error!(error = %e, "create permission group failed");
            Status::InternalServerError
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[put("/permissions/groups/<group_id>", data = "<body>")]
pub async fn update_permission_group(
    cap: CapabilityGuard,
    state: &State<AppState>,
    group_id: String,
    body: Json<PermissionGroupInput>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let body = body.into_inner();
    validate_group_input(&body)?;
    let gid: uuid::Uuid = group_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "permissions.admin").await?;
            // Special preset roles (superadmin/superuser) are system-defined:
            // their grants are fixed, so editing via this endpoint would let an
            // admin silently weaken or redirect them.
            let special: Option<(String,)> = sqlx::query_as(
                "SELECT special FROM sec.\"group\" WHERE tenant_id=$1 AND group_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(gid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db)?;
            match special {
                None => {
                    return Err(DbError::Other {
                        message: "HTTP404:group".into(),
                    });
                }
                Some((s,)) if !s.is_empty() => {
                    return Err(DbError::Other {
                        message: "HTTP422:special role is not editable".into(),
                    });
                }
                _ => {}
            }
            let parent: Option<uuid::Uuid> = body
                .parent_id
                .as_deref()
                .filter(|s| !s.trim().is_empty())
                .and_then(|s| s.parse().ok());
            validate_parent_group(&mut tx, ctx.tenant_id, parent).await?;
            let changed = sqlx::query(
                "UPDATE sec.\"group\" SET alias=$3, description=$4, class=$5, parent_id=$6
                 WHERE tenant_id=$1 AND group_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(gid)
            .bind(body.name.trim())
            .bind(&body.description)
            .bind(&body.class)
            .bind(parent)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
            if changed == 0 {
                return Err(DbError::Other {
                    message: "HTTP404:group".into(),
                });
            }
            write_group(&mut tx, ctx.tenant_id, gid, &body).await?;
            tx.commit().await.map_err(db)?;
            let mut tx = audit_tx(&pool, &ctx, "permissions.read").await?;
            let groups = load_groups(&mut tx, ctx.tenant_id).await?;
            Ok(groups
                .into_iter()
                .find(|g| g["id"].as_str() == Some(&gid.to_string()))
                .unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| match e {
            DbError::Other { message } if message == "HTTP404:group" => Status::NotFound,
            DbError::Other { message } if message.starts_with("HTTP422:") => {
                tracing::warn!(detail = %message, "permission group rejected");
                Status::UnprocessableEntity
            }
            _ => Status::InternalServerError,
        })?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[rocket::delete("/permissions/groups/<group_id>")]
pub async fn delete_permission_group(
    cap: CapabilityGuard,
    state: &State<AppState>,
    group_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let gid: uuid::Uuid = group_id.parse().map_err(|_| Status::NotFound)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    exec.run(Deadline::in_secs(15), move |pool| async move {
        let mut tx = audit_tx(&pool, &ctx, "permissions.admin").await?;
        // Special preset roles are system-defined and must not be deleted.
        let special: Option<(String,)> =
            sqlx::query_as("SELECT special FROM sec.\"group\" WHERE tenant_id=$1 AND group_id=$2")
                .bind(ctx.tenant_id)
                .bind(gid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db)?;
        match special {
            Some((s,)) if !s.is_empty() => {
                return Err(DbError::Other {
                    message: "HTTP409:special role cannot be deleted".into(),
                });
            }
            _ => {}
        }
        // A role with inheriting children cannot be deleted: children would
        // silently lose the parent rule set. Remove the inheritance link or
        // the children first.
        let children: Option<(i64,)> = sqlx::query_as(
            "SELECT count(*) FROM sec.\"group\" WHERE tenant_id=$1 AND parent_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(gid)
        .fetch_optional(&mut *tx)
        .await
        .map_err(db)?;
        if children.map(|(c,)| c).unwrap_or(0) > 0 {
            return Err(DbError::Other {
                message: "HTTP409:group has inheriting roles".into(),
            });
        }
        sqlx::query(
            "DELETE FROM sec.principal_group_membership WHERE tenant_id=$1 AND group_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(gid)
        .execute(&mut *tx)
        .await
        .map_err(db)?;
        sqlx::query("DELETE FROM sec.policy_rule WHERE tenant_id=$1 AND group_id=$2")
            .bind(ctx.tenant_id)
            .bind(gid)
            .execute(&mut *tx)
            .await
            .map_err(db)?;
        let changed = sqlx::query("DELETE FROM sec.\"group\" WHERE tenant_id=$1 AND group_id=$2")
            .bind(ctx.tenant_id)
            .bind(gid)
            .execute(&mut *tx)
            .await
            .map_err(db)?
            .rows_affected();
        if changed == 0 {
            return Err(DbError::Other {
                message: "HTTP404:group".into(),
            });
        }
        tx.commit().await.map_err(db)?;
        Ok(Value::Null)
    })
    .await
    .map_err(|e| match e {
        DbError::Other { message } if message == "HTTP404:group" => Status::NotFound,
        DbError::Other { message } if message.starts_with("HTTP409:") => Status::Conflict,
        _ => Status::InternalServerError,
    })?;
    Ok(Json(ApiEnvelope {
        data: json!({ "deleted": true }),
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        query_audit,
        latest_checkpoint,
        create_checkpoint,
        verify_chain,
        get_retention,
        put_retention,
        list_permission_groups,
        create_permission_group,
        update_permission_group,
        delete_permission_group,
    ]
}
