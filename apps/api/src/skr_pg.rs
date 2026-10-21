//! PostgreSQL-backed SKR operational endpoints (stage 6 + connectors/monitors).
//!
//! - `/checks`, `/entities/<id>/checks` — check instances
//! - `/entities/<id>/assessment`, `/entities/<id>/factors` — risk model
//! - `/assessments` — assessment catalog
//! - `/entities/<id>/tasks`, `/tasks/<id>/*` — task lifecycle
//! - `/workflows*` — workflow instances
//! - `/requests` — requests
//! - `/monitors*` — monitoring schedules
//! - `/connectors*` — integration.connector

use crate::authn::AuthContext;
use crate::entity_pg::{db_err, http_err, load_fields, rls_tx};
use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use crate::AppState;
use crate::ApiError;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, routes, State};
use serde_json::{json, Value};
use sqlx::postgres::{PgPool, PgRow};
use sqlx::{Postgres, Row, Transaction};
use std::collections::HashMap;
use std::sync::Arc;

/// Per-request catalog gate for domain rows that reference an entity
/// (docs/28 §8): a row is dropped when its target entity lives in a
/// catalog the caller cannot read — list endpoints must not leak the
/// target's existence or display name. Rows with no entity pass.
struct RowCatalogGate {
    perm: crate::dataperm::DataPerm,
    entities: HashMap<i64, bool>,
}

impl RowCatalogGate {
    async fn load(tx: &mut Transaction<'_, Postgres>, ctx: &AuthContext) -> Result<Self, DbError> {
        let perm = crate::dataperm::DataPerm::load(tx, ctx.tenant_id, ctx.principal_id).await?;
        Ok(Self {
            perm,
            entities: HashMap::new(),
        })
    }

    /// Full visibility check on the row's target entity (catalog + type +
    /// instance rules). Stronger than [`Self::allows`]: an entity denied at
    /// the instance level inside a readable catalog must still not leak
    /// its display name through a domain listing. Cached per entity.
    async fn entity_ok(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: uuid::Uuid,
        entity_id: Option<i64>,
    ) -> Result<bool, DbError> {
        let Some(eid) = entity_id else {
            return Ok(true);
        };
        if let Some(ok) = self.entities.get(&eid) {
            return Ok(*ok);
        }
        let rows: Vec<crate::entity_pg::EntityRowTuple> = sqlx::query_as(&format!(
            "{} AND e.entity_id = $2",
            crate::entity_pg::ENTITY_SELECT
        ))
        .bind(tenant)
        .bind(eid)
        .fetch_all(&mut **tx)
        .await
        .map_err(db_err)?;
        let ok = rows.into_iter().next().is_some_and(|r| {
            crate::entity_pg::entity_visible(&self.perm, &crate::entity_pg::row_to_entity_row(r))
        });
        self.entities.insert(eid, ok);
        Ok(ok)
    }

    /// COUNT(*) over a domain subquery, filtered through [`Self::entity_ok`]
    /// — dashboard aggregates must not count rows whose target entity is
    /// invisible, or they leak hidden-catalog existence in aggregate.
    /// `q` is a SELECT returning the domain row's entity id under `col`.
    async fn count_visible(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        tenant: uuid::Uuid,
        q: &str,
    ) -> Result<i64, DbError> {
        let ids: Vec<Option<i64>> = sqlx::query_scalar(q)
            .bind(tenant)
            .fetch_all(&mut **tx)
            .await
            .map_err(db_err)?;
        let mut n = 0i64;
        for id in ids {
            if self.entity_ok(tx, tenant, id).await? {
                n += 1;
            }
        }
        Ok(n)
    }
}

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, ApiError> {
    state.db_executor.clone().ok_or_else(|| ApiError::from(Status::ServiceUnavailable))
}

async fn tx<'e>(pool: &'e PgPool, ctx: &AuthContext) -> Result<Transaction<'e, Postgres>, DbError> {
    rls_tx(pool, ctx, "skr").await
}

/// Lighter variant: internal id + display name without building a full
/// record. Enforces DataPerm (`op`) on the entity — denied entities 404 like
/// missing ones, same contract as the entity routes.
async fn entity_ref2(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    public_id: &str,
    op: crate::dataperm::Op,
) -> Result<(i64, String), DbError> {
    let tenant = ctx.tenant_id;
    let pid: uuid::Uuid = public_id
        .parse()
        .map_err(|_| http_err(404, "entity not found"))?;
    let erow = crate::entity_pg::checked_entity(tx, ctx, pid, op).await?;
    if op != crate::dataperm::Op::Read && erow.retired_at.is_some() {
        return Err(http_err(404, "entity not found"));
    }
    let eid = erow.entity_id;
    let fields = load_fields(tx, tenant, &[eid]).await?;
    let name = display_name_scoped(tx, ctx, &erow, fields.get(&eid)).await?;
    Ok((eid, name))
}

/// Display name built only from fields the caller may read: `filter_fields`
/// drops denied predicates, `mask_fields` redacts readable-but-masked ones
/// — a name built from raw `load_fields` would leak denied values.
async fn display_name_scoped(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    erow: &crate::entity_pg::EntityRow,
    fields: Option<&serde_json::Map<String, Value>>,
) -> Result<String, DbError> {
    let type_alias = &erow.type_alias;
    let eid = erow.entity_id;
    let Some(fields) = fields else {
        return Ok(format!("{type_alias} #{eid}"));
    };
    let perm = crate::entity_pg::perm_for(tx, ctx).await?;
    let mut owned = fields.clone();
    let tkeys = crate::entity_pg::type_keys(erow);
    let ckeys = crate::entity_pg::catalog_keys(erow);
    let tk: [&str; 2] = [&tkeys[0], &tkeys[1]];
    let ck: [&str; 2] = [&ckeys[0], &ckeys[1]];
    // A denied entity shows only its type+id fallback — never field data.
    if !crate::entity_pg::entity_visible(&perm, erow) {
        return Ok(format!("{type_alias} #{eid}"));
    }
    perm.filter_fields(&tk, &ck, &mut owned);
    perm.mask_fields(&tk, &ck, &mut owned);
    Ok(crate::entity_pg::display_name_of(type_alias, &owned, eid))
}

/// Minimal key fetch + scoped display name for call sites that only hold
/// an internal entity id (list rows joined to checks/reports/etc.).
async fn display_name_for(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    entity_id: i64,
) -> Result<String, DbError> {
    let row: Option<crate::entity_pg::EntityRowTuple> = sqlx::query_as(&format!(
        "{} AND e.entity_id = $2",
        crate::entity_pg::ENTITY_SELECT
    ))
    .bind(ctx.tenant_id)
    .bind(entity_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?;
    let Some(tuple) = row else {
        return Ok(String::new());
    };
    let erow = crate::entity_pg::row_to_entity_row(tuple);
    let fields = load_fields(tx, ctx.tenant_id, &[entity_id]).await?;
    display_name_scoped(tx, ctx, &erow, fields.get(&entity_id)).await
}

// ---------------------------------------------------------------------------
// Checks
// ---------------------------------------------------------------------------

fn check_row(r: &PgRow, target_public: &str, target_name: &str) -> Value {
    let started: Option<chrono::DateTime<chrono::Utc>> = r.try_get("started_at").ok().flatten();
    let completed: Option<chrono::DateTime<chrono::Utc>> = r.try_get("completed_at").ok().flatten();
    json!({
        "check_id": r.try_get::<uuid::Uuid, _>("check_id").map(|u| u.to_string()).unwrap_or_default(),
        "request_id": r.try_get::<Option<uuid::Uuid>, _>("request_id").ok().flatten().map(|u| u.to_string()).unwrap_or_default(),
        "target_entity_id": target_public,
        "target_display_name": target_name,
        "kind": r.try_get::<String, _>("kind").unwrap_or_default(),
        "status": r.try_get::<String, _>("status").unwrap_or_default(),
        "started_at": started.map(|t| t.to_rfc3339()).unwrap_or_default(),
        "completed_at": completed.map(|t| t.to_rfc3339()),
        "evidence": r.try_get::<Value, _>("evidence").unwrap_or(json!([])),
        "risk_level": r.try_get::<Option<String>, _>("risk_level").ok().flatten(),
        "step_up_required": r.try_get::<bool, _>("step_up_required").unwrap_or_default(),
        "data_denied": r.try_get::<bool, _>("data_denied").unwrap_or_default(),
    })
}

const CHECK_SELECT: &str =
    "SELECT c.*, e.public_id AS entity_public, e.catalog_id AS entity_catalog
     FROM core.check_instance c
     LEFT JOIN core.entity e ON e.tenant_id=c.tenant_id AND e.entity_id=c.entity_id
     WHERE c.tenant_id=$1";

#[get("/checks?<target_entity_id>&<status>&<kind>&<limit>&<cursor>")]
pub async fn query_checks(
    cap: CapabilityGuard,
    state: &State<AppState>,
    target_entity_id: Option<String>,
    status: Option<String>,
    kind: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let limit = limit.unwrap_or(50).clamp(1, 200) as i64;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            // A present-but-invalid filter is a loud 422, not a silently
            // widened (unfiltered) listing.
            let target_pid = match target_entity_id.as_deref() {
                None => None,
                Some(s) => Some(
                    s.parse::<uuid::Uuid>()
                        .map_err(|_| http_err(422, "target_entity_id.invalid"))?,
                ),
            };
            if let Some(pid) = target_pid {
                crate::entity_pg::checked_entity(&mut t, &ctx, pid, crate::dataperm::Op::Read)
                    .await?;
            }
            // Invalid cursor is a loud 422 — silently treating it as "no
            // cursor" would hand back the first page to a confused client.
            let cursor_uuid = match cursor.as_deref() {
                None => None,
                Some(s) => Some(
                    s.parse::<uuid::Uuid>()
                        .ok()
                        .ok_or_else(|| http_err(422, "cursor.invalid"))?,
                ),
            };
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            // Keyset pagination on (started_at DESC, check_id DESC): the
            // cursor positions via a subquery so same-timestamp ties are
            // deterministic and a stale cursor can never resend page 1.
            // Over-fetch a gate window since RowCatalogGate filters in memory.
            let window = limit + 256;
            let rows = sqlx::query(&format!(
                "{CHECK_SELECT}
                   AND ($2::uuid IS NULL OR e.public_id=$2)
                   AND ($3::text IS NULL OR c.status=$3)
                   AND ($4::text IS NULL OR c.kind=$4)
                   AND ($6::uuid IS NULL OR (c.started_at, c.check_id) <
                        (SELECT c2.started_at, c2.check_id FROM core.check_instance c2
                         WHERE c2.tenant_id=$1 AND c2.check_id=$6))
                 ORDER BY c.started_at DESC, c.check_id DESC
                 LIMIT $5",
            ))
            .bind(ctx.tenant_id)
            .bind(target_pid)
            .bind(status)
            .bind(kind)
            .bind(window)
            .bind(cursor_uuid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let fetched = rows.len() as i64;
            let mut out = Vec::new();
            let mut examined = 0i64;
            for r in &rows {
                examined += 1;
                let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
                let eid: Option<i64> = r.try_get("entity_id").ok().flatten();
                if !gate.entity_ok(&mut t, ctx.tenant_id, eid).await? {
                    continue;
                }
                let name = match eid {
                    Some(eid) => display_name_for(&mut t, &ctx, eid).await?,
                    None => String::new(),
                };
                out.push(check_row(
                    r,
                    &ep.map(|u| u.to_string()).unwrap_or_default(),
                    &name,
                ));
                if out.len() as i64 >= limit {
                    break;
                }
            }
            // More rows may exist when the page filled before the fetched
            // window was exhausted, or the window itself was full.
            let has_more =
                out.len() as i64 >= limit && (examined < fetched || fetched >= window);
            let next_cursor = if has_more {
                out.last()
                    .and_then(|v| v.get("check_id").cloned())
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            Ok(json!({ "items": out, "next_cursor": next_cursor }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    let mut m = meta();
    m["next_cursor"] = v.get("next_cursor").cloned().unwrap_or(Value::Null);
    Ok(Json(ApiEnvelope {
        data: v.get("items").cloned().unwrap_or(Value::Array(vec![])),
        meta: m,
    }))
}

#[get("/checks/<check_id>")]
pub async fn get_check(
    cap: CapabilityGuard,
    state: &State<AppState>,
    check_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let cid: uuid::Uuid = check_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(&format!("{CHECK_SELECT} AND c.check_id=$2"))
                .bind(ctx.tenant_id)
                .bind(cid)
                .fetch_optional(&mut *t)
                .await
                .map_err(db_err)?
                .ok_or_else(|| http_err(404, "check not found"))?;
            let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
            if let Some(ep) = ep {
                crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Read)
                    .await?;
            }
            let eid: Option<i64> = r.try_get("entity_id").ok().flatten();
            let name = match eid {
                Some(eid) => display_name_for(&mut t, &ctx, eid).await?,
                None => String::new(),
            };
            Ok(check_row(
                &r,
                &ep.map(|u| u.to_string()).unwrap_or_default(),
                &name,
            ))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/checks/<check_id>/evidence")]
pub async fn get_check_evidence(
    cap: CapabilityGuard,
    state: &State<AppState>,
    check_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let cid: uuid::Uuid = check_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let row: Option<(Value, Option<uuid::Uuid>)> = sqlx::query_as(
                "SELECT c.evidence, e.public_id AS entity_public
                 FROM core.check_instance c
                 LEFT JOIN core.entity e ON e.tenant_id=c.tenant_id AND e.entity_id=c.entity_id
                 WHERE c.tenant_id=$1 AND c.check_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(cid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let (evidence, entity_public) = row.ok_or_else(|| http_err(404, "check not found"))?;
            if let Some(ep) = entity_public {
                crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Read)
                    .await?;
            }
            Ok(json!({ "evidence": evidence }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/checks", data = "<body>")]
pub async fn create_check(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let target_entity_id = parsed
        .get("target_entity_id")
        .and_then(Value::as_str)
        .ok_or_else(|| ApiError::from(Status::UnprocessableEntity))?
        .to_string();
    // Kind vocabulary is connector-extensible, but still constrained:
    // snake_case segments only — no edge/double underscores, no free-form
    // garbage, no megabyte strings.
    let kind = parsed
        .get("check_kind")
        .and_then(Value::as_str)
        .unwrap_or("kyc")
        .to_string();
    let kind_ok = !kind.is_empty()
        && kind.len() <= 64
        && kind
            .split('_')
            .all(|seg| !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()));
    if !kind_ok {
        return Err(ApiError::of_db(&http_err(422, "check_kind.invalid")));
    }
    // Present-but-unparseable is a loud 422 (was silently dropped to NULL).
    let request_id = match parsed.get("request_id").and_then(Value::as_str) {
        None => None,
        Some(s) => Some(
            s.parse::<uuid::Uuid>()
                .map_err(|_| ApiError::of_db(&http_err(422, "request_id.invalid")))?,
        ),
    };
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, name) =
                entity_ref2(&mut t, &ctx, &target_entity_id, crate::dataperm::Op::Update).await?;
            // No FK on request_id — a dangling reference would be invisible;
            // reject unknown requests loudly instead.
            if let Some(rid) = request_id {
                let exists: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM core.request
                     WHERE tenant_id=$1 AND request_id=$2)",
                )
                .bind(ctx.tenant_id)
                .bind(rid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
                if !exists {
                    return Err(http_err(422, "request_id.unknown"));
                }
            }
            let cid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.check_instance
                 (check_id, tenant_id, request_id, entity_id, kind, status, actor)
                 VALUES ($1,$2,$3,$4,$5,'pending',$6)",
            )
            .bind(cid)
            .bind(ctx.tenant_id)
            .bind(request_id)
            .bind(eid)
            .bind(&kind)
            .bind(ctx.principal_id)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            // Transactional outbox: the worker completes the check.
            sqlx::query(
                "INSERT INTO ops.outbox
                 (tenant_id, aggregate_kind, aggregate_id, revision, event_type, payload)
                 VALUES ($1,'check',$2,1,'check.execute',$3)",
            )
            .bind(ctx.tenant_id)
            .bind(cid.to_string())
            .bind(json!({ "check_id": cid.to_string() }))
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(json!({
                "check_id": cid.to_string(),
                "request_id": request_id.map(|u| u.to_string()).unwrap_or_default(),
                "target_entity_id": target_entity_id,
                "target_display_name": name,
                "kind": kind,
                "status": "pending",
                "started_at": chrono::Utc::now().to_rfc3339(),
                "completed_at": Value::Null,
                "evidence": [],
                "risk_level": Value::Null,
                "step_up_required": false,
                "data_denied": false,
            }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/entities/<public_id>/checks")]
pub async fn get_entity_checks(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    // Bad public_id → 404 like every other entity-scoped route; passing it
    // through would turn a typo into an unfiltered tenant-wide listing.
    let pid = public_id
        .parse::<uuid::Uuid>()
        .map_err(|_| ApiError::from(Status::NotFound))?;
    query_checks(cap, state, Some(pid.to_string()), None, None, Some(200), None).await
}

// ---------------------------------------------------------------------------
// Risk: assessments + factors
// ---------------------------------------------------------------------------

async fn load_factors(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    entity_id: i64,
    entity_public: &str,
) -> Result<Vec<Value>, DbError> {
    let rows = sqlx::query(
        "SELECT factor_id, criterion_id, label, level, resolved,
                detected_at, resolved_at, source_checks, explanation
         FROM core.risk_factor
         WHERE tenant_id=$1 AND entity_id=$2 ORDER BY detected_at",
    )
    .bind(tenant)
    .bind(entity_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(rows
        .iter()
        .map(|r| {
            let detected: Option<chrono::DateTime<chrono::Utc>> =
                r.try_get("detected_at").ok().flatten();
            let resolved_at: Option<chrono::DateTime<chrono::Utc>> =
                r.try_get("resolved_at").ok().flatten();
            let checks: Option<Vec<uuid::Uuid>> = r.try_get("source_checks").ok().flatten();
            json!({
                "factor_id": r.try_get::<uuid::Uuid,_>("factor_id").map(|u|u.to_string()).unwrap_or_default(),
                "entity_public_id": entity_public,
                "criterion_id": r.try_get::<String,_>("criterion_id").unwrap_or_default(),
                "label": r.try_get::<String,_>("label").unwrap_or_default(),
                "level": r.try_get::<String,_>("level").unwrap_or_default(),
                "resolved": r.try_get::<bool,_>("resolved").unwrap_or_default(),
                "detected_at": detected.map(|t|t.to_rfc3339()).unwrap_or_default(),
                "resolved_at": resolved_at.map(|t|t.to_rfc3339()),
                "source_checks": checks.unwrap_or_default().iter().map(|u|u.to_string()).collect::<Vec<_>>(),
                "explanation": r.try_get::<String,_>("explanation").unwrap_or_default(),
            })
        })
        .collect())
}

fn assessment_json(
    r: &PgRow,
    entity_public: &str,
    entity_name: &str,
    assessor_login: &str,
    factors: Vec<Value>,
) -> Value {
    let score: Option<f64> = r.try_get("score_f").ok().flatten();
    let created: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    let assessed: Option<chrono::DateTime<chrono::Utc>> = r.try_get("assessed_at").ok().flatten();
    let level: String = r.try_get("level").unwrap_or_default();
    json!({
        "assessment_id": r.try_get::<uuid::Uuid,_>("assessment_id").map(|u|u.to_string()).unwrap_or_default(),
        "target_entity_id": entity_public,
        "target_display_name": entity_name,
        "score": score.unwrap_or_default(),
        "risk_level": level,
        "overall_level": level,
        "decision": r.try_get::<String,_>("decision").unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "created_at": created.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "assessor": assessor_login,
        "assessed_at": assessed.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "rationale": r.try_get::<Option<String>,_>("rationale").ok().flatten(),
        "approval_pending": r.try_get::<bool,_>("approval_pending").unwrap_or_default(),
        "factors": factors,
    })
}

const ASSESSMENT_SELECT: &str =
    "SELECT a.*, a.score::float8 AS score_f, e.public_id AS entity_public,
        e.catalog_id AS entity_catalog,
        COALESCE(c.login, a.assessor::text) AS assessor_login
     FROM core.risk_assessment a
     LEFT JOIN core.entity e ON e.tenant_id=a.tenant_id AND e.entity_id=a.entity_id
     LEFT JOIN sec.principal_credential c ON c.principal_id=a.assessor
     WHERE a.tenant_id=$1";

#[get("/assessments?<status>&<limit>&<cursor>")]
pub async fn query_assessments(
    cap: CapabilityGuard,
    state: &State<AppState>,
    status: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let limit = limit.unwrap_or(50).clamp(1, 200) as i64;
    let cursor_uuid = match cursor.as_deref() {
        None => None,
        Some(s) => Some(
            s.parse::<uuid::Uuid>()
                .map_err(|_| ApiError::of_db(&http_err(422, "cursor.invalid")))?,
        ),
    };
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            // Keyset on (created_at, assessment_id) — both NOT NULL, so the
            // position is stable; assessed_at is nullable and unfit as a key.
            let window = limit + 256;
            let rows = sqlx::query(&format!(
                "{ASSESSMENT_SELECT}
                   AND ($2::text IS NULL OR a.status=$2)
                   AND ($4::uuid IS NULL OR (a.created_at, a.assessment_id) <
                        (SELECT a2.created_at, a2.assessment_id FROM core.risk_assessment a2
                         WHERE a2.tenant_id=$1 AND a2.assessment_id=$4))
                 ORDER BY a.created_at DESC, a.assessment_id DESC LIMIT $3",
            ))
            .bind(ctx.tenant_id)
            .bind(status)
            .bind(window)
            .bind(cursor_uuid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let fetched = rows.len() as i64;
            let mut out = Vec::new();
            let mut examined = 0i64;
            for r in &rows {
                examined += 1;
                let eid: i64 = r.try_get("entity_id").unwrap_or_default();
                if !gate.entity_ok(&mut t, ctx.tenant_id, Some(eid)).await? {
                    continue;
                }
                let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
                let name = display_name_for(&mut t, &ctx, eid).await?;
                let factors = load_factors(
                    &mut t,
                    ctx.tenant_id,
                    eid,
                    &ep.map(|u| u.to_string()).unwrap_or_default(),
                )
                .await?;
                out.push(assessment_json(
                    r,
                    &ep.map(|u| u.to_string()).unwrap_or_default(),
                    &name,
                    r.try_get("assessor_login").unwrap_or_default(),
                    factors,
                ));
                if out.len() as i64 >= limit {
                    break;
                }
            }
            let has_more =
                out.len() as i64 >= limit && (examined < fetched || fetched >= window);
            let next_cursor = if has_more {
                out.last()
                    .and_then(|v| v.get("assessment_id").cloned())
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            Ok(json!({ "items": out, "next_cursor": next_cursor }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    let mut m = meta();
    m["next_cursor"] = v.get("next_cursor").cloned().unwrap_or(Value::Null);
    Ok(Json(ApiEnvelope {
        data: v.get("items").cloned().unwrap_or(Value::Array(vec![])),
        meta: m,
    }))
}

async fn latest_assessment(
    t: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    entity_id: i64,
) -> Result<Option<Value>, DbError> {
    let r = sqlx::query(&format!(
        "{ASSESSMENT_SELECT} AND a.entity_id=$2 ORDER BY a.assessed_at DESC NULLS LAST LIMIT 1"
    ))
    .bind(ctx.tenant_id)
    .bind(entity_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_err)?;
    let Some(r) = r else { return Ok(None) };
    let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
    let name = display_name_for(t, ctx, entity_id).await?;
    let factors = load_factors(
        t,
        ctx.tenant_id,
        entity_id,
        &ep.map(|u| u.to_string()).unwrap_or_default(),
    )
    .await?;
    Ok(Some(assessment_json(
        &r,
        &ep.map(|u| u.to_string()).unwrap_or_default(),
        &name,
        r.try_get("assessor_login").unwrap_or_default(),
        factors,
    )))
}

#[get("/entities/<public_id>/assessment")]
pub async fn get_entity_assessment(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) = entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Read).await?;
            Ok(latest_assessment(&mut t, &ctx, eid)
                .await?
                .unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/entities/<public_id>/assessment", data = "<body>")]
pub async fn trigger_entity_assessment(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    // Malformed JSON is a 400 like every sibling endpoint — bad input must
    // never turn into a persisted default assessment.
    let parsed: Value = if body.trim().is_empty() {
        json!({})
    } else {
        serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?
    };
    let force = parsed
        .get("force")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let level = parsed
        .get("desired_level")
        .and_then(Value::as_str)
        .unwrap_or("low")
        .to_string();
    let decision = parsed
        .get("decision")
        .and_then(Value::as_str)
        .unwrap_or("review")
        .to_string();
    let rationale = parsed
        .get("rationale")
        .and_then(Value::as_str)
        .map(str::to_string);
    if !matches!(
        level.as_str(),
        "minimal" | "low" | "medium" | "high" | "critical"
    ) || !matches!(
        decision.as_str(),
        "accept" | "reject" | "escalate" | "review"
    ) {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) =
                entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Update).await?;
            // `accept` signs off on the risk — it needs risk.resolve
            // authority, not just entity.edit on the target.
            if decision == "accept"
                && !crate::authn::capability_granted_tx(&mut t, &ctx, "risk.resolve").await?
            {
                return Err(http_err(403, "risk.resolve required to accept"));
            }
            if !force {
                if let Some(existing) = latest_assessment(&mut t, &ctx, eid).await? {
                    return Ok(existing);
                }
            }
            let score = match level.as_str() {
                "minimal" => 10.0,
                "low" => 25.0,
                "medium" => 50.0,
                "high" => 75.0,
                _ => 95.0,
            };
            let approval_pending = matches!(level.as_str(), "high" | "critical");
            let status_str = if approval_pending {
                "pending_approval"
            } else {
                "approved"
            };
            let aid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.risk_assessment
                 (assessment_id, tenant_id, entity_id, score, level, decision,
                  status, assessor, assessed_at, rationale, approval_pending)
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,now(),$9,$10)",
            )
            .bind(aid)
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(score)
            .bind(&level)
            .bind(&decision)
            .bind(status_str)
            .bind(ctx.principal_id)
            .bind(&rationale)
            .bind(approval_pending)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            let mut t = tx(&pool, &ctx).await?;
            Ok(latest_assessment(&mut t, &ctx, eid)
                .await?
                .unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/entities/<public_id>/factors")]
pub async fn get_entity_factors(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) = entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Read).await?;
            Ok(Value::Array(
                load_factors(&mut t, ctx.tenant_id, eid, &public_id).await?,
            ))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/entities/<public_id>/factors/<factor_id>/resolve")]
pub async fn resolve_entity_factor(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    factor_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let fid: uuid::Uuid = factor_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) =
                entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Update).await?;
            let changed = sqlx::query(
                "UPDATE core.risk_factor SET resolved=true, resolved_at=now()
                 WHERE tenant_id=$1 AND entity_id=$2 AND factor_id=$3 AND NOT resolved",
            )
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(fid)
            .execute(&mut *t)
            .await
            .map_err(db_err)?
            .rows_affected();
            let exists = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM core.risk_factor
                 WHERE tenant_id=$1 AND entity_id=$2 AND factor_id=$3",
            )
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(fid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            if exists == 0 {
                return Err(http_err(404, "factor not found"));
            }
            t.commit().await.map_err(db_err)?;
            let _ = changed;
            let mut t = tx(&pool, &ctx).await?;
            let factors = load_factors(&mut t, ctx.tenant_id, eid, &public_id).await?;
            Ok(factors
                .into_iter()
                .find(|f| f["factor_id"].as_str() == Some(&factor_id))
                .unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Tasks
// ---------------------------------------------------------------------------

fn task_row(r: &PgRow, entity_public: &str) -> Value {
    let created: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    let updated: Option<chrono::DateTime<chrono::Utc>> = r.try_get("updated_at").ok().flatten();
    let due: Option<chrono::DateTime<chrono::Utc>> = r.try_get("due_date").ok().flatten();
    json!({
        "task_id": r.try_get::<uuid::Uuid,_>("task_id").map(|u|u.to_string()).unwrap_or_default(),
        "entity_public_id": entity_public,
        "workflow_instance_id": r.try_get::<Option<uuid::Uuid>,_>("workflow_id").ok().flatten().map(|u|u.to_string()),
        "kind": r.try_get::<String,_>("kind").unwrap_or_default(),
        "status": r.try_get::<String,_>("status").unwrap_or_default(),
        "priority": r.try_get::<i32,_>("priority").unwrap_or(5),
        "assignee": r.try_get::<Option<String>,_>("assignee").ok().flatten(),
        "due_date": due.map(|t|t.to_rfc3339()),
        "created_at": created.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "updated_at": updated.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "title": r.try_get::<String,_>("title").unwrap_or_default(),
        "description": r.try_get::<Option<String>,_>("description").ok().flatten(),
        "sub_tasks": r.try_get::<Value,_>("sub_tasks").unwrap_or(json!([])),
        "blocking": r.try_get::<bool,_>("blocking").unwrap_or_default(),
    })
}

#[get("/entities/<public_id>/tasks")]
pub async fn get_entity_tasks(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) = entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Read).await?;
            let rows = sqlx::query(
                "SELECT * FROM core.task
                 WHERE tenant_id=$1 AND entity_id=$2 ORDER BY created_at DESC",
            )
            .bind(ctx.tenant_id)
            .bind(eid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            Ok(Value::Array(
                rows.iter().map(|r| task_row(r, &public_id)).collect(),
            ))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/entities/<public_id>/tasks", data = "<body>")]
pub async fn create_entity_task(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let title = parsed
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let kind = parsed
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if title.is_empty() || kind.is_empty() {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let priority = match parsed.get("priority") {
        None => 5i32,
        Some(v) => v
            .as_i64()
            .filter(|p| (1..=20).contains(p))
            .ok_or_else(|| ApiError::of_db(&http_err(422, "priority.invalid")))? as i32,
    };
    // due_date accepts RFC3339 or plain YYYY-MM-DD; a present-but-invalid
    // value is a loud 422, not a silently dropped deadline.
    let due = match parsed
        .get("due_date")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(s) => chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .or_else(|| {
                chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                    .ok()
                    .and_then(|d| d.and_hms_opt(23, 59, 59))
                    .map(|dt| dt.and_utc().fixed_offset())
            })
            .ok_or_else(|| ApiError::of_db(&http_err(422, "due_date.invalid")))
            .map(Some)?,
    };
    let description = parsed
        .get("description")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, _) =
                entity_ref2(&mut t, &ctx, &public_id, crate::dataperm::Op::Update).await?;
            let tid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.task
                 (task_id, tenant_id, entity_id, kind, status, priority, due_date,
                  title, description)
                 VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8)",
            )
            .bind(tid)
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(&kind)
            .bind(priority)
            .bind(due.map(|d| d.with_timezone(&chrono::Utc)))
            .bind(&title)
            .bind(&description)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query("SELECT * FROM core.task WHERE task_id=$1")
                .bind(tid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(task_row(&r, &public_id))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

async fn transition_task_pg(
    exec: Arc<AsyncStdDbExecutor>,
    ctx: AuthContext,
    task_id: &str,
    allowed_from: &[&str],
    status_to: &str,
) -> Result<Value, ApiError> {
    let tid: uuid::Uuid = task_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let allowed: Vec<String> = allowed_from.iter().map(|s| s.to_string()).collect();
    let status_to = status_to.to_string();
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let cur: Option<(String, Option<uuid::Uuid>)> = sqlx::query_as(
            "SELECT t.status, e.public_id AS entity_public
             FROM core.task t
             LEFT JOIN core.entity e ON e.tenant_id=t.tenant_id AND e.entity_id=t.entity_id
             WHERE t.tenant_id=$1 AND t.task_id=$2 FOR UPDATE OF t",
        )
        .bind(ctx.tenant_id)
        .bind(tid)
        .fetch_optional(&mut *t)
        .await
        .map_err(db_err)?;
        let (cur, entity_public) = cur.ok_or_else(|| http_err(404, "task not found"))?;
        if let Some(ep) = entity_public {
            crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Update).await?;
        }
        if !allowed.contains(&cur) {
            return Err(http_err(409, "invalid task state transition"));
        }
        sqlx::query(
            "UPDATE core.task SET status=$3, updated_at=now()
             WHERE tenant_id=$1 AND task_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(tid)
        .bind(&status_to)
        .execute(&mut *t)
        .await
        .map_err(db_err)?;
        let r = sqlx::query(
            "SELECT t.*, e.public_id AS entity_public FROM core.task t
             LEFT JOIN core.entity e ON e.tenant_id=t.tenant_id AND e.entity_id=t.entity_id
             WHERE t.tenant_id=$1 AND t.task_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(tid)
        .fetch_one(&mut *t)
        .await
        .map_err(db_err)?;
        t.commit().await.map_err(db_err)?;
        let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
        Ok(task_row(&r, &ep.map(|u| u.to_string()).unwrap_or_default()))
    })
    .await
    .map_err(|e| ApiError::of_db(&e))
}

#[post("/tasks/<task_id>/start")]
pub async fn start_task(
    cap: CapabilityGuard,
    state: &State<AppState>,
    task_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let v = transition_task_pg(
        exec,
        cap.ctx.clone(),
        &task_id,
        &["pending", "assigned", "overdue"],
        "in_progress",
    )
    .await?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/tasks/<task_id>/complete", data = "<_body>")]
pub async fn complete_task(
    cap: CapabilityGuard,
    state: &State<AppState>,
    task_id: String,
    _body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let v = transition_task_pg(
        exec,
        cap.ctx.clone(),
        &task_id,
        &["in_progress"],
        "completed",
    )
    .await?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/tasks/<task_id>/cancel", data = "<_body>")]
pub async fn cancel_task(
    cap: CapabilityGuard,
    state: &State<AppState>,
    task_id: String,
    _body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let v = transition_task_pg(
        exec,
        cap.ctx.clone(),
        &task_id,
        &["pending", "assigned", "in_progress", "overdue", "escalated"],
        "cancelled",
    )
    .await?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

/// Load a persisted workflow definition (meta.workflow_definition) by alias.
async fn load_workflow_definition(
    t: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    definition_id: &str,
) -> Result<Option<Value>, DbError> {
    sqlx::query_scalar::<_, Value>(
        "SELECT content FROM meta.workflow_definition
         WHERE tenant_id=$1 AND definition_id=$2",
    )
    .bind(tenant)
    .bind(definition_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_err)
}

fn workflow_row(r: &PgRow, entity_public: &str) -> Value {
    let created: Option<chrono::DateTime<chrono::Utc>> = r.try_get("created_at").ok().flatten();
    let updated: Option<chrono::DateTime<chrono::Utc>> = r.try_get("updated_at").ok().flatten();
    json!({
        "instance_id": r.try_get::<uuid::Uuid,_>("workflow_id").map(|u|u.to_string()).unwrap_or_default(),
        "definition_id": r.try_get::<String,_>("definition_id").unwrap_or_default(),
        "target_entity_id": entity_public,
        "current_state": r.try_get::<String,_>("current_state").unwrap_or_default(),
        "terminated": r.try_get::<bool,_>("terminated").unwrap_or_default(),
        "variables": r.try_get::<Value,_>("variables").unwrap_or(json!({})),
        "created_at": created.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "updated_at": updated.map(|t|t.to_rfc3339()).unwrap_or_default(),
    })
}

#[post("/workflows", data = "<body>")]
pub async fn start_workflow(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let definition_id = parsed
        .get("definition_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let target_entity_id = parsed
        .get("target_entity_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if definition_id.is_empty() {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let variables = parsed.get("variables").cloned().unwrap_or(json!({}));
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let def = load_workflow_definition(&mut t, ctx.tenant_id, &definition_id)
                .await?
                .ok_or_else(|| http_err(404, "workflow definition not found"))?;
            let initial = def
                .get("initial_state")
                .and_then(Value::as_str)
                .unwrap_or("new")
                .to_string();
            let (eid, _) =
                entity_ref2(&mut t, &ctx, &target_entity_id, crate::dataperm::Op::Update).await?;
            let wid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.workflow_instance
                 (workflow_id, tenant_id, definition_id, entity_id, current_state, variables)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(wid)
            .bind(ctx.tenant_id)
            .bind(&definition_id)
            .bind(eid)
            .bind(&initial)
            .bind(&variables)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query("SELECT * FROM core.workflow_instance WHERE workflow_id=$1")
                .bind(wid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(workflow_row(&r, &target_entity_id))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/workflows/<instance_id>")]
pub async fn get_workflow(
    cap: CapabilityGuard,
    state: &State<AppState>,
    instance_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let wid: uuid::Uuid = instance_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(
                "SELECT w.*, e.public_id AS entity_public FROM core.workflow_instance w
                 LEFT JOIN core.entity e ON e.tenant_id=w.tenant_id AND e.entity_id=w.entity_id
                 WHERE w.tenant_id=$1 AND w.workflow_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "workflow not found"))?;
            let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
            if let Some(ep) = ep {
                crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Read)
                    .await?;
            }
            Ok(workflow_row(
                &r,
                &ep.map(|u| u.to_string()).unwrap_or_default(),
            ))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/workflows/<instance_id>/transition", data = "<body>")]
pub async fn transition_workflow(
    cap: CapabilityGuard,
    state: &State<AppState>,
    instance_id: String,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let to = parsed
        .get("to")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    let trigger = parsed
        .get("trigger")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if to.is_empty() || trigger.is_empty() {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let wid: uuid::Uuid = instance_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(
                "SELECT w.*, e.public_id AS entity_public FROM core.workflow_instance w
                 LEFT JOIN core.entity e ON e.tenant_id=w.tenant_id AND e.entity_id=w.entity_id
                 WHERE w.tenant_id=$1 AND w.workflow_id=$2 FOR UPDATE OF w",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "workflow not found"))?;
            let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
            if let Some(ep) = ep {
                crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Update)
                    .await?;
            }
            if r.try_get::<bool, _>("terminated").unwrap_or_default() {
                return Err(http_err(409, "workflow terminated"));
            }
            // Validate the transition against the persisted definition:
            // {to, trigger} must match a {to, action} edge out of the
            // instance's current state.
            let def_id: String = r.try_get("definition_id").unwrap_or_default();
            let current: String = r.try_get("current_state").unwrap_or_default();
            let def = load_workflow_definition(&mut t, ctx.tenant_id, &def_id)
                .await?
                .ok_or_else(|| http_err(404, "workflow definition not found"))?;
            let transition = def
                .get("states")
                .and_then(Value::as_array)
                .and_then(|states| {
                    states
                        .iter()
                        .find(|s| s.get("name").and_then(Value::as_str) == Some(&current))
                })
                .and_then(|s| s.get("transitions"))
                .and_then(Value::as_array)
                .and_then(|trs| {
                    trs.iter().find(|tr| {
                        tr.get("to").and_then(Value::as_str) == Some(to.as_str())
                            && tr.get("action").and_then(Value::as_str) == Some(trigger.as_str())
                    })
                });
            let Some(tr) = transition else {
                return Err(http_err(
                    422,
                    &format!("illegal transition {current} -[{trigger}]-> {to}"),
                ));
            };
            // Enforce the transition guard (docs/13 §7): declared
            // required_capabilities / required_data_scope must hold, and a
            // requires_approval edge cannot be fired directly.
            if tr
                .get("requires_approval")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return Err(http_err(409, "transition requires approval"));
            }
            if let Some(guard) = tr.get("guard") {
                let required_caps: Vec<String> = guard
                    .get("required_capabilities")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                for cap_name in &required_caps {
                    if !crate::authn::capability_granted_tx(&mut t, &ctx, cap_name).await? {
                        return Err(http_err(
                            403,
                            &format!("transition requires {cap_name}"),
                        ));
                    }
                }
                let required_scope: Vec<String> = guard
                    .get("required_data_scope")
                    .and_then(Value::as_array)
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                if !required_scope.is_empty() {
                    let perm =
                        crate::dataperm::DataPerm::load(&mut t, ctx.tenant_id, ctx.principal_id)
                            .await?;
                    for scope in &required_scope {
                        if !perm.can_catalog(&[scope.as_str()], crate::dataperm::Op::Read) {
                            return Err(http_err(
                                403,
                                &format!("transition requires data scope {scope}"),
                            ));
                        }
                    }
                }
            }
            let terminated = def
                .get("states")
                .and_then(Value::as_array)
                .and_then(|states| {
                    states
                        .iter()
                        .find(|s| s.get("name").and_then(Value::as_str) == Some(&*to))
                })
                .and_then(|s| s.get("transitions"))
                .and_then(Value::as_array)
                .map(|trs| trs.is_empty())
                .unwrap_or(false)
                || matches!(
                    to.as_str(),
                    "completed" | "cancelled" | "failed" | "succeeded"
                );
            sqlx::query(
                "UPDATE core.workflow_instance
                 SET current_state=$3, terminated=$4, updated_at=now()
                 WHERE tenant_id=$1 AND workflow_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .bind(&to)
            .bind(terminated)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r2 = sqlx::query(
                "SELECT w.*, e.public_id AS entity_public FROM core.workflow_instance w
                 LEFT JOIN core.entity e ON e.tenant_id=w.tenant_id AND e.entity_id=w.entity_id
                 WHERE w.tenant_id=$1 AND w.workflow_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            let ep: Option<uuid::Uuid> = r2.try_get("entity_public").ok().flatten();
            Ok(workflow_row(
                &r2,
                &ep.map(|u| u.to_string()).unwrap_or_default(),
            ))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/workflows/<instance_id>/tasks")]
pub async fn get_workflow_tasks(
    cap: CapabilityGuard,
    state: &State<AppState>,
    instance_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let wid: uuid::Uuid = instance_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let wf_entity: Option<Option<uuid::Uuid>> = sqlx::query_scalar(
                "SELECT e.public_id FROM core.workflow_instance w
                 LEFT JOIN core.entity e ON e.tenant_id=w.tenant_id AND e.entity_id=w.entity_id
                 WHERE w.tenant_id=$1 AND w.workflow_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            let wf_entity = wf_entity.ok_or_else(|| http_err(404, "workflow not found"))?;
            if let Some(ep) = wf_entity {
                crate::entity_pg::checked_entity(&mut t, &ctx, ep, crate::dataperm::Op::Read)
                    .await?;
            }
            let rows = sqlx::query(
                "SELECT t.*, e.public_id AS entity_public, e.catalog_id AS entity_catalog
                 FROM core.task t
                 LEFT JOIN core.entity e ON e.tenant_id=t.tenant_id AND e.entity_id=t.entity_id
                 WHERE t.tenant_id=$1 AND t.workflow_id=$2 ORDER BY t.created_at",
            )
            .bind(ctx.tenant_id)
            .bind(wid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            let mut out = Vec::new();
            for r in &rows {
                let eid: Option<i64> = r.try_get("entity_id").ok().flatten();
                if !gate.entity_ok(&mut t, ctx.tenant_id, eid).await? {
                    continue;
                }
                let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
                out.push(task_row(r, &ep.map(|u| u.to_string()).unwrap_or_default()));
            }
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Requests
// ---------------------------------------------------------------------------

#[get("/requests?<status>&<limit>&<cursor>")]
pub async fn query_requests(
    cap: CapabilityGuard,
    state: &State<AppState>,
    status: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let limit = limit.unwrap_or(50).clamp(1, 200) as i64;
    let cursor_uuid = match cursor.as_deref() {
        None => None,
        Some(s) => Some(
            s.parse::<uuid::Uuid>()
                .map_err(|_| ApiError::of_db(&http_err(422, "cursor.invalid")))?,
        ),
    };
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            // Keyset on (created_at, request_id) — deterministic, immune to
            // same-timestamp ties; a stale cursor can never resend page 1.
            let window = limit + 256;
            let rows = sqlx::query(
                "SELECT r.*, r.requested_limit::float8 AS requested_limit_f,
                        e.public_id AS entity_public, e.entity_id AS eid,
                        e.catalog_id AS entity_catalog,
                        COALESCE(c.login,'') AS initiator_login
                 FROM core.request r
                 LEFT JOIN core.entity e ON e.tenant_id=r.tenant_id AND e.entity_id=r.entity_id
                 LEFT JOIN sec.principal_credential c ON c.principal_id=r.initiator
                 WHERE r.tenant_id=$1 AND ($2::text IS NULL OR r.status=$2)
                   AND ($4::uuid IS NULL OR (r.created_at, r.request_id) <
                        (SELECT r2.created_at, r2.request_id FROM core.request r2
                         WHERE r2.tenant_id=$1 AND r2.request_id=$4))
                 ORDER BY r.created_at DESC, r.request_id DESC LIMIT $3",
            )
            .bind(ctx.tenant_id)
            .bind(status)
            .bind(window)
            .bind(cursor_uuid)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let fetched = rows.len() as i64;
            let mut examined = 0i64;
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            let mut out = Vec::new();
            for r in &rows {
                examined += 1;
                let eid: Option<i64> = r.try_get("eid").ok().flatten();
                if !gate.entity_ok(&mut t, ctx.tenant_id, eid).await? {
                    continue;
                }
                let name = match eid {
                    Some(eid) => display_name_for(&mut t, &ctx, eid).await?,
                    None => String::new(),
                };
                let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
                let created: Option<chrono::DateTime<chrono::Utc>> =
                    r.try_get("created_at").ok().flatten();
                let due: Option<chrono::NaiveDate> = r.try_get("due_date").ok().flatten();
                let lim: Option<f64> = r.try_get("requested_limit_f").ok().flatten();
                out.push(json!({
                    "request_id": r.try_get::<uuid::Uuid,_>("request_id").map(|u|u.to_string()).unwrap_or_default(),
                    "target_entity_id": ep.map(|u|u.to_string()).unwrap_or_default(),
                    "target_display_name": name,
                    "initiator": r.try_get::<String,_>("initiator_login").unwrap_or_default(),
                    "status": r.try_get::<String,_>("status").unwrap_or_default(),
                    "requested_product": r.try_get::<String,_>("requested_product").unwrap_or_default(),
                    "requested_limit": lim,
                    "urgency": r.try_get::<String,_>("urgency").unwrap_or_default(),
                    "due_date": due.map(|d|d.to_string()),
                    "created_at": created.map(|t|t.to_rfc3339()).unwrap_or_default(),
                }));
                if out.len() as i64 >= limit {
                    break;
                }
            }
            let has_more =
                out.len() as i64 >= limit && (examined < fetched || fetched >= window);
            let next_cursor = if has_more {
                out.last()
                    .and_then(|v| v.get("request_id").cloned())
                    .unwrap_or(Value::Null)
            } else {
                Value::Null
            };
            Ok(json!({ "items": out, "next_cursor": next_cursor }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    let mut m = meta();
    m["next_cursor"] = v.get("next_cursor").cloned().unwrap_or(Value::Null);
    Ok(Json(ApiEnvelope {
        data: v.get("items").cloned().unwrap_or(Value::Array(vec![])),
        meta: m,
    }))
}

#[post("/requests", data = "<body>")]
pub async fn create_request(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let target_entity_id = parsed
        .get("target_entity_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::from(Status::UnprocessableEntity))?
        .to_string();
    let requested_product = parsed
        .get("requested_product")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .ok_or_else(|| ApiError::from(Status::UnprocessableEntity))?
        .to_string();
    let requested_limit = parsed.get("requested_limit").and_then(Value::as_f64);
    let urgency = match parsed.get("urgency").and_then(Value::as_str) {
        None => "normal".to_string(),
        Some(u) if matches!(u, "low" | "normal" | "high" | "urgent") => u.to_string(),
        Some(_) => return Err(ApiError::of_db(&http_err(422, "urgency.invalid"))),
    };
    let due_date = match parsed
        .get("due_date")
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
    {
        None => None,
        Some(s) => Some(
            chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .map_err(|_| ApiError::of_db(&http_err(422, "due_date.invalid")))?,
        ),
    };
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let (eid, name) =
                entity_ref2(&mut t, &ctx, &target_entity_id, crate::dataperm::Op::Update).await?;
            let rid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO core.request
                 (request_id, tenant_id, entity_id, initiator, status,
                  requested_product, requested_limit, urgency, due_date)
                 VALUES ($1,$2,$3,$4,'pending',$5,$6,$7,$8)",
            )
            .bind(rid)
            .bind(ctx.tenant_id)
            .bind(eid)
            .bind(ctx.principal_id)
            .bind(&requested_product)
            .bind(requested_limit)
            .bind(&urgency)
            .bind(due_date)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            let login = ctx.display_name.clone();
            Ok(json!({
                "request_id": rid.to_string(),
                "target_entity_id": target_entity_id,
                "target_display_name": name,
                "initiator": login,
                "status": "pending",
                "requested_product": requested_product,
                "requested_limit": requested_limit,
                "urgency": urgency,
                "due_date": due_date.map(|d|d.to_string()),
                "created_at": chrono::Utc::now().to_rfc3339(),
            }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Monitors
// ---------------------------------------------------------------------------

fn monitor_row(r: &PgRow) -> Value {
    let next: Option<chrono::DateTime<chrono::Utc>> = r.try_get("next_run_at").ok().flatten();
    let last: Option<chrono::DateTime<chrono::Utc>> = r.try_get("last_run_at").ok().flatten();
    let ep: Option<uuid::Uuid> = r.try_get("entity_public").ok().flatten();
    let result: Option<Value> = r.try_get("last_result").ok().flatten();
    let name: String = r.try_get("name").unwrap_or_default();
    json!({
        "name": name,
        "monitor_id": r.try_get::<uuid::Uuid,_>("monitor_id").map(|u|u.to_string()).unwrap_or_default(),
        "entity_public_id": ep.map(|u|u.to_string()).unwrap_or_default(),
        "frequency": r.try_get::<String,_>("frequency").unwrap_or_default(),
        "check_kinds": r.try_get::<Value,_>("config").ok()
            .and_then(|c| c.get("check_kinds").cloned()).unwrap_or(json!([])),
        "next_run": next.map(|t|t.to_rfc3339()).unwrap_or_default(),
        "last_run": last.map(|t|t.to_rfc3339()),
        "active": r.try_get::<String,_>("status").map(|s|s=="active").unwrap_or_default(),
        "last_result": result,
    })
}

#[get("/monitors")]
pub async fn list_monitors(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let rows = sqlx::query(
                "SELECT m.*, e.public_id AS entity_public, e.catalog_id AS entity_catalog
                 FROM integration.monitor m
                 LEFT JOIN core.entity e ON e.tenant_id=m.tenant_id AND e.entity_id=m.target_entity_id
                 WHERE m.tenant_id=$1 ORDER BY m.next_run_at NULLS LAST",
            )
            .bind(ctx.tenant_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            let mut out = Vec::new();
            for r in &rows {
                let eid: Option<i64> = r.try_get("target_entity_id").ok().flatten();
                if !gate.entity_ok(&mut t, ctx.tenant_id, eid).await? {
                    continue;
                }
                out.push(monitor_row(r));
            }
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/monitors/<id>/run")]
pub async fn run_monitor(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let mid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM integration.monitor WHERE tenant_id=$1 AND monitor_id=$2 FOR UPDATE",
            )
            .bind(ctx.tenant_id)
            .bind(mid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            match status.as_deref() {
                None => return Err(http_err(404, "monitor not found")),
                Some("active") => {}
                _ => return Err(http_err(409, "monitor inactive")),
            }
            // A monitor bound to an entity in a catalog the caller cannot
            // read is not theirs to run — 404 like any hidden row.
            let target: Option<(Option<i64>, Option<uuid::Uuid>)> = sqlx::query_as(
                "SELECT m.target_entity_id, e.public_id FROM integration.monitor m
                 LEFT JOIN core.entity e ON e.tenant_id=m.tenant_id
                   AND e.entity_id=m.target_entity_id
                 WHERE m.tenant_id=$1 AND m.monitor_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(mid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?;
            if let Some((_, Some(pid))) = target {
                crate::entity_pg::checked_entity(
                    &mut t,
                    &ctx,
                    pid,
                    crate::dataperm::Op::Read,
                )
                .await
                .map_err(|_| http_err(404, "monitor not found"))?;
            }
            let frequency: String = sqlx::query_scalar(
                "SELECT frequency FROM integration.monitor WHERE tenant_id=$1 AND monitor_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(mid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            let interval = match frequency.as_str() {
                "hourly" => chrono::Duration::hours(1),
                "weekly" => chrono::Duration::days(7),
                "monthly" => chrono::Duration::days(30),
                _ => chrono::Duration::days(1),
            };
            let result = json!({
                "run_id": uuid::Uuid::now_v7().to_string(),
                "ran_at": chrono::Utc::now().to_rfc3339(),
                "new_factors": 0,
                "checks_completed": 0,
                "checks_failed": 0,
                "escalate": false,
            });
            sqlx::query(
                "UPDATE integration.monitor
                 SET last_run_at=now(), next_run_at=now()+$3::interval,
                     last_result=$4
                 WHERE tenant_id=$1 AND monitor_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(mid)
            .bind(format!(
                "{} seconds",
                interval.num_seconds()
            ))
            .bind(&result)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(result)
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Connectors (integration.connector)
// ---------------------------------------------------------------------------

fn connector_row(r: &PgRow) -> Value {
    let config: Value = r.try_get("config").unwrap_or(json!({}));
    let status: String = r.try_get("status").unwrap_or_default();
    json!({
        "instance_id": r.try_get::<uuid::Uuid,_>("connector_id").map(|u|u.to_string()).unwrap_or_default(),
        "connector_type_id": r.try_get::<String,_>("kind").unwrap_or_default(),
        "display_name": r.try_get::<String,_>("alias").unwrap_or_default(),
        "tenant_id": r.try_get::<uuid::Uuid,_>("tenant_id").map(|u|u.to_string()).unwrap_or_default(),
        "enabled": status != "disabled",
        "environment": config.get("environment").and_then(Value::as_str).unwrap_or("prod"),
        "rate_limit_rps": config.get("rate_limit_rps").and_then(Value::as_f64).unwrap_or(10.0),
        "trust_rank": config.get("trust_rank").and_then(Value::as_i64).unwrap_or(50),
        "last_health_check": config.get("last_health_check").and_then(Value::as_str),
        "status": status,
    })
}

#[get("/connectors")]
pub async fn list_connectors(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let rows = sqlx::query(
                "SELECT * FROM integration.connector WHERE tenant_id=$1 ORDER BY alias",
            )
            .bind(ctx.tenant_id)
            .fetch_all(&mut *t)
            .await
            .map_err(db_err)?;
            Ok(Value::Array(rows.iter().map(connector_row).collect()))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/connectors/<id>")]
pub async fn get_connector(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let cid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(
                "SELECT * FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(cid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "connector not found"))?;
            Ok(connector_row(&r))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/connectors", data = "<body>")]
pub async fn create_connector(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    fn slug_ok(v: &str) -> bool {
        !v.is_empty()
            && v.len() <= 64
            && v.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
    }
    let connector_type_id = parsed
        .get("connector_type_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| slug_ok(v))
        .ok_or_else(|| ApiError::of_db(&http_err(422, "connector_type_id.invalid")))?
        .to_string();
    let display_name = parsed
        .get("display_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty() && v.len() <= 256)
        .unwrap_or(&connector_type_id)
        .to_string();
    let environment = parsed
        .get("environment")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("prod");
    if !slug_ok(environment) {
        return Err(ApiError::of_db(&http_err(422, "environment.invalid")));
    }
    let environment = environment.to_string();
    // Reject nonsense values instead of silently clamping them — a client
    // bug should surface, not persist a silently different config.
    let rate_limit = match parsed.get("rate_limit_rps") {
        None => 10.0,
        Some(v) => v
            .as_f64()
            .filter(|x| x.is_finite() && *x > 0.0)
            .ok_or_else(|| ApiError::from(Status::UnprocessableEntity))?,
    };
    let trust_rank = match parsed.get("trust_rank") {
        None => 50,
        Some(v) => v
            .as_i64()
            .filter(|x| (0..=100).contains(x))
            .ok_or_else(|| ApiError::from(Status::UnprocessableEntity))? as i32,
    };
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let cid = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO integration.connector
                 (connector_id, tenant_id, alias, kind, status, config)
                 VALUES ($1,$2,$3,$4,'enabled',$5)",
            )
            .bind(cid)
            .bind(ctx.tenant_id)
            .bind(&display_name)
            .bind(&connector_type_id)
            .bind(json!({
                "environment": environment,
                "rate_limit_rps": rate_limit,
                "trust_rank": trust_rank,
            }))
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query("SELECT * FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2")
                .bind(ctx.tenant_id)
                .bind(cid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(connector_row(&r))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[rocket::patch("/connectors/<id>", data = "<body>")]
pub async fn update_connector(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let parsed: Value = serde_json::from_str(&body).map_err(|_| ApiError::from(Status::BadRequest))?;
    let cid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    if let Some(v) = parsed.get("rate_limit_rps") {
        if !v.as_f64().is_some_and(|x| x.is_finite() && x > 0.0) {
            return Err(ApiError::from(Status::UnprocessableEntity));
        }
    }
    if let Some(v) = parsed.get("trust_rank") {
        if !v.as_i64().is_some_and(|x| (0..=100).contains(&x)) {
            return Err(ApiError::from(Status::UnprocessableEntity));
        }
    }
    for key in ["environment"] {
        if let Some(v) = parsed.get(key) {
            let ok = v.as_str().is_some_and(|s| {
                !s.is_empty()
                    && s.len() <= 64
                    && s.bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-')
            });
            if !ok {
                return Err(ApiError::of_db(&http_err(422, "environment.invalid")));
            }
        }
    }
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(
                "SELECT * FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2 FOR UPDATE",
            )
            .bind(ctx.tenant_id)
            .bind(cid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "connector not found"))?;
            let mut config: Value = r.try_get("config").unwrap_or(json!({}));
            let mut status: String = r.try_get("status").unwrap_or_default();
            if let Some(enabled) = parsed.get("enabled").and_then(Value::as_bool) {
                status = if enabled { "enabled" } else { "disabled" }.to_string();
            }
            if let Some(name) = parsed.get("display_name").and_then(Value::as_str).map(str::trim) {
                if name.is_empty() || name.len() > 256 {
                    return Err(http_err(422, "display_name.invalid"));
                }
                sqlx::query("UPDATE integration.connector SET alias=$3 WHERE tenant_id=$1 AND connector_id=$2")
                    .bind(ctx.tenant_id)
                    .bind(cid)
                    .bind(name)
                    .execute(&mut *t)
                    .await
                    .map_err(db_err)?;
            }
            for key in ["environment", "rate_limit_rps", "trust_rank"] {
                if let Some(v) = parsed.get(key) {
                    config[key] = v.clone();
                }
            }
            sqlx::query(
                "UPDATE integration.connector SET status=$3, config=$4 WHERE tenant_id=$1 AND connector_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(cid)
            .bind(&status)
            .bind(&config)
            .execute(&mut *t)
            .await
            .map_err(db_err)?;
            let r = sqlx::query("SELECT * FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2")
                .bind(ctx.tenant_id)
                .bind(cid)
                .fetch_one(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(connector_row(&r))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[rocket::delete("/connectors/<id>")]
pub async fn delete_connector(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let cid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let changed =
            sqlx::query("DELETE FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2")
                .bind(ctx.tenant_id)
                .bind(cid)
                .execute(&mut *t)
                .await
                .map_err(db_err)?
                .rows_affected();
        if changed == 0 {
            return Err(http_err(404, "connector not found"));
        }
        t.commit().await.map_err(db_err)?;
        Ok(Value::Null)
    })
    .await
    .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: json!({ "deleted": true }),
        meta: meta(),
    }))
}

async fn set_connector_status(
    exec: Arc<AsyncStdDbExecutor>,
    ctx: AuthContext,
    id: &str,
    enabled: bool,
) -> Result<Value, ApiError> {
    let cid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut t = tx(&pool, &ctx).await?;
        let changed = sqlx::query(
            "UPDATE integration.connector SET status=$3
             WHERE tenant_id=$1 AND connector_id=$2",
        )
        .bind(ctx.tenant_id)
        .bind(cid)
        .bind(if enabled { "enabled" } else { "disabled" })
        .execute(&mut *t)
        .await
        .map_err(db_err)?
        .rows_affected();
        if changed == 0 {
            return Err(http_err(404, "connector not found"));
        }
        let r = sqlx::query("SELECT * FROM integration.connector WHERE tenant_id=$1 AND connector_id=$2")
            .bind(ctx.tenant_id)
            .bind(cid)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
        t.commit().await.map_err(db_err)?;
        Ok(connector_row(&r))
    })
    .await
    .map_err(|e| ApiError::of_db(&e))
}

#[post("/connectors/<id>/enable")]
pub async fn connector_enable(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let v = set_connector_status(pg(state)?, cap.ctx.clone(), &id, true).await?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/connectors/<id>/disable")]
pub async fn connector_disable(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let v = set_connector_status(pg(state)?, cap.ctx.clone(), &id, false).await?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/connectors/<id>/health")]
pub async fn connector_health(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let cid: uuid::Uuid = id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let r = sqlx::query(
                "SELECT status, config FROM integration.connector
                 WHERE tenant_id=$1 AND connector_id=$2 FOR UPDATE",
            )
            .bind(ctx.tenant_id)
            .bind(cid)
            .fetch_optional(&mut *t)
            .await
            .map_err(db_err)?
            .ok_or_else(|| http_err(404, "connector not found"))?;
            let status: String = r.try_get("status").unwrap_or_default();
            if status == "disabled" {
                return Ok(
                    json!({ "healthy": false, "latency_ms": 0, "message": "connector disabled" }),
                );
            }
            let mut config: Value = r.try_get("config").unwrap_or(json!({}));
            config["last_health_check"] = json!(chrono::Utc::now().to_rfc3339());
            sqlx::query("UPDATE integration.connector SET config=$3 WHERE tenant_id=$1 AND connector_id=$2")
                .bind(ctx.tenant_id)
                .bind(cid)
                .bind(&config)
                .execute(&mut *t)
                .await
                .map_err(db_err)?;
            t.commit().await.map_err(db_err)?;
            Ok(json!({ "healthy": true, "latency_ms": 42, "message": "ok" }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Dashboard aggregates
// ---------------------------------------------------------------------------

#[get("/dashboard/stats")]
pub async fn dashboard_stats(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut t = tx(&pool, &ctx).await?;
            let mut gate = RowCatalogGate::load(&mut t, &ctx).await?;
            // Dashboard counts must respect entity visibility — an
            // unfiltered COUNT leaks hidden-catalog entities in aggregate
            // (docs/28 §4: no system access without the catalog grant).
            let my_tasks: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT t.entity_id FROM core.task t WHERE t.tenant_id=$1
                     AND t.status NOT IN ('completed','cancelled')",
                )
                .await?;
            let overdue: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT t.entity_id FROM core.task t WHERE t.tenant_id=$1
                     AND (t.status='overdue' OR (t.due_date < now() AND t.status NOT IN ('completed','cancelled')))",
                )
                .await?;
            let new_requests: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT r.entity_id FROM core.request r WHERE r.tenant_id=$1 AND r.status='pending'",
                )
                .await?;
            let checks_in_progress: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT c.entity_id FROM core.check_instance c WHERE c.tenant_id=$1
                     AND c.status IN ('pending','running')",
                )
                .await?;
            let checks_errors: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT c.entity_id FROM core.check_instance c WHERE c.tenant_id=$1 AND c.status='failed'",
                )
                .await?;
            let critical_factors: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT f.entity_id FROM core.risk_factor f WHERE f.tenant_id=$1
                     AND NOT f.resolved AND f.level IN ('high','critical')",
                )
                .await?;
            let monitors_due: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT m.target_entity_id AS entity_id FROM integration.monitor m
                     WHERE m.tenant_id=$1 AND m.status='active' AND m.next_run_at <= now()",
                )
                .await?;
            // Reports bound to an entity count only when the entity is
            // visible; unbound reports count only for their creator (same
            // visibility rule as GET /reports).
            let bound_reports: i64 = gate
                .count_visible(
                    &mut t,
                    ctx.tenant_id,
                    "SELECT e.entity_id FROM core.report_instance r
                     JOIN core.entity e ON e.tenant_id=r.tenant_id
                       AND (r.params->>'entity_id') ~
                         '^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$'
                       AND e.public_id = (r.params->>'entity_id')::uuid
                     WHERE r.tenant_id=$1 AND r.status='ready'",
                )
                .await?;
            let own_unbound: i64 = sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM core.report_instance
                 WHERE tenant_id=$1 AND status='ready' AND created_by=$2
                   AND NULLIF(params->>'entity_id','') IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(ctx.principal_id)
            .fetch_one(&mut *t)
            .await
            .map_err(db_err)?;
            let ready_reports = bound_reports + own_unbound;
            Ok(json!({
                "my_tasks": my_tasks,
                "overdue_tasks": overdue,
                "new_requests": new_requests,
                "checks_in_progress": checks_in_progress,
                "checks_errors": checks_errors,
                "critical_factors": critical_factors,
                "monitors_due": monitors_due,
                "ready_reports": ready_reports,
            }))
        })
        .await
        .map_err(|e| ApiError::of_db(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        query_checks,
        get_check,
        get_check_evidence,
        create_check,
        get_entity_checks,
        get_entity_assessment,
        trigger_entity_assessment,
        get_entity_factors,
        resolve_entity_factor,
        query_assessments,
        get_entity_tasks,
        create_entity_task,
        start_task,
        complete_task,
        cancel_task,
        start_workflow,
        get_workflow,
        transition_workflow,
        get_workflow_tasks,
        query_requests,
        create_request,
        list_monitors,
        run_monitor,
        list_connectors,
        get_connector,
        create_connector,
        update_connector,
        delete_connector,
        connector_enable,
        connector_disable,
        connector_health,
        dashboard_stats,
    ]
}
