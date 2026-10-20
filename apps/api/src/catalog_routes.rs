//! Catalog registry routes (docs/28): `core.catalog` CRUD.
//!
//! - `GET    /catalogs`        — list catalogs the principal can read.
//! - `GET    /catalogs/<ref>`  — one catalog by alias or public_id.
//! - `POST   /catalogs`        — create a custom catalog.
//! - `PUT    /catalogs/<ref>`  — rename / describe / change settings,
//!                               `status: retired` retires the catalog.
//! - `POST   /catalogs/<ref>/retire` — retire shortcut (409 when non-empty).
//!
//! Capability mapping lives in `workspace_routes::required_capabilities`:
//! reads need `entity.read`, mutations `catalog.manage` (umbrella
//! `security.manage`). On top of that every object-level access goes
//! through `DataPerm::can_catalog` (deny-by-default, docs/28 §4).

use crate::entity_pg::{db_err, http_err, perm_for, resolve_catalog, rls_tx, to_status};
use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use crate::AppState;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, put, State};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use std::sync::Arc;

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, Status> {
    state.db_executor.clone().ok_or(Status::ServiceUnavailable)
}

/// Same alias contract as the DDL check `catalog_alias_chk`
/// (`^[a-z][a-z0-9_-]{0,62}$`) — validated here to return 422 instead of a
/// constraint violation.
fn valid_alias(alias: &str) -> bool {
    let bytes = alias.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 63
        && bytes[0].is_ascii_lowercase()
        && bytes
            .iter()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || *b == b'_' || *b == b'-')
}

type CatalogRow = (
    i64,
    uuid::Uuid,
    String,
    String,
    String,
    String,
    chrono::DateTime<chrono::Utc>,
    Option<chrono::DateTime<chrono::Utc>>,
);

/// `resolve_catalog` reports a missing reference as `catalog.unknown`
/// (422) — inside `/catalogs/<ref>` the resource itself is absent, so the
/// REST contract is 404 instead.
fn not_found_if_unknown(e: DbError) -> DbError {
    if let DbError::Other { message } = &e {
        if message.contains("catalog.unknown") {
            return http_err(404, "catalog not found");
        }
    }
    e
}

fn catalog_json(row: &CatalogRow, entity_count: i64, type_count: i64) -> Value {
    json!({
        "catalog_id": row.0,
        "public_id": row.1.to_string(),
        "alias": row.2,
        "name": row.3,
        "description": row.4,
        "status": row.5,
        "created_at": row.6.to_rfc3339(),
        "retired_at": row.7.map(|t| t.to_rfc3339()),
        "entity_count": entity_count,
        "type_count": type_count,
    })
}

/// Live entity / type counts for the record payload.
async fn catalog_counts(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog_id: i64,
) -> Result<(i64, i64), DbError> {
    let (entities,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM core.entity
         WHERE tenant_id=$1 AND catalog_id=$2 AND retired_at IS NULL",
    )
    .bind(tenant)
    .bind(catalog_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    let (types,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM meta.type_definition
         WHERE tenant_id=$1 AND catalog_id=$2",
    )
    .bind(tenant)
    .bind(catalog_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok((entities, types))
}

#[get("/catalogs")]
pub async fn list_catalogs(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "catalogs.list").await?;
            let perm = perm_for(&mut tx, &ctx).await?;
            let rows: Vec<CatalogRow> = sqlx::query_as(
                "SELECT catalog_id, public_id, alias, name, description, status,
                        created_at, retired_at
                 FROM core.catalog WHERE tenant_id=$1 ORDER BY alias",
            )
            .bind(ctx.tenant_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let mut out = Vec::new();
            for row in &rows {
                let pid = row.1.to_string();
                if !perm.can_catalog(&[&pid, &row.2], crate::dataperm::Op::Read) {
                    continue;
                }
                let (entities, types) = catalog_counts(&mut tx, ctx.tenant_id, row.0).await?;
                out.push(catalog_json(row, entities, types));
            }
            tx.commit().await.map_err(db_err)?;
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/catalogs/<reference>")]
pub async fn get_catalog(
    cap: CapabilityGuard,
    state: &State<AppState>,
    reference: &str,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let reference = reference.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "catalogs.get").await?;
            let cat = resolve_catalog(&mut tx, ctx.tenant_id, &reference)
                .await
                .map_err(not_found_if_unknown)?;
            let perm = perm_for(&mut tx, &ctx).await?;
            let keys = cat.keys();
            let key_refs: [&str; 2] = [&keys[0], &keys[1]];
            // An inaccessible catalog looks identical to a missing one.
            if !perm.can_catalog(&key_refs, crate::dataperm::Op::Read) {
                return Err(http_err(404, "catalog not found"));
            }
            let row: Option<CatalogRow> = sqlx::query_as(
                "SELECT catalog_id, public_id, alias, name, description, status,
                        created_at, retired_at
                 FROM core.catalog WHERE tenant_id=$1 AND catalog_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(cat.catalog_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let row = row.ok_or_else(|| http_err(404, "catalog not found"))?;
            let (entities, types) = catalog_counts(&mut tx, ctx.tenant_id, cat.catalog_id).await?;
            tx.commit().await.map_err(db_err)?;
            Ok(catalog_json(&row, entities, types))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct CreateCatalogBody {
    pub alias: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub settings: Value,
}

#[post("/catalogs", data = "<body>")]
pub async fn create_catalog(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: Json<CreateCatalogBody>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let body = body.into_inner();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "catalogs.create").await?;
            let alias = body.alias.trim().to_string();
            if !valid_alias(&alias) {
                return Err(http_err(422, "catalog.alias_invalid"));
            }
            if body.name.trim().is_empty() {
                return Err(http_err(422, "catalog.name_required"));
            }
            // Reserved namespaces: `sys`/`general` exist per tenant and
            // `data.catalog.create` is scoped by the wildcard catalog key —
            // creating a catalog is itself a catalog permission.
            let perm = perm_for(&mut tx, &ctx).await?;
            if !perm.can_catalog(&["*"], crate::dataperm::Op::Create) {
                return Err(http_err(403, "catalog create denied"));
            }
            let row: Option<CatalogRow> = sqlx::query_as(
                "INSERT INTO core.catalog
                    (tenant_id, catalog_id, public_id, alias, name, description,
                     settings, created_by)
                 VALUES ($1, nextval('core.catalog_id_seq'), gen_random_uuid(),
                         $2, $3, $4, $5, $6)
                 ON CONFLICT (tenant_id, alias) DO NOTHING
                 RETURNING catalog_id, public_id, alias, name, description,
                           status, created_at, retired_at",
            )
            .bind(ctx.tenant_id)
            .bind(&alias)
            .bind(body.name.trim())
            .bind(&body.description)
            .bind(&body.settings)
            .bind(ctx.principal_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let row = row.ok_or_else(|| http_err(409, "catalog.alias_taken"))?;
            tx.commit().await.map_err(db_err)?;
            Ok(catalog_json(&row, 0, 0))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateCatalogBody {
    pub name: Option<String>,
    pub description: Option<String>,
    pub settings: Option<Value>,
    /// `active` reactivates, `retired` retires (blocked while the catalog
    /// still holds live entities — retire or move them first).
    pub status: Option<String>,
}

async fn update_catalog_row(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &crate::authn::AuthContext,
    reference: &str,
    body: &UpdateCatalogBody,
) -> Result<CatalogRow, DbError> {
    let cat = resolve_catalog(tx, ctx.tenant_id, reference)
        .await
        .map_err(not_found_if_unknown)?;
    let perm = perm_for(tx, ctx).await?;
    let keys = cat.keys();
    let key_refs: [&str; 2] = [&keys[0], &keys[1]];
    if !perm.can_catalog(&key_refs, crate::dataperm::Op::Update) {
        return Err(http_err(404, "catalog not found"));
    }
    // `sys`/`general` are seeded infrastructure: metadata may change but
    // their lifecycle is fixed — retiring them would orphan the platform.
    if matches!(cat.alias.as_str(), "sys" | "general") && body.status.as_deref() == Some("retired")
    {
        return Err(http_err(422, "catalog.system_immutable"));
    }
    if let Some(status) = body.status.as_deref() {
        if !matches!(status, "active" | "retired") {
            return Err(http_err(422, "catalog.status_invalid"));
        }
        if status == "retired" {
            let (live,): (i64,) = sqlx::query_as(
                "SELECT count(*) FROM core.entity
                 WHERE tenant_id=$1 AND catalog_id=$2 AND retired_at IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(cat.catalog_id)
            .fetch_one(&mut **tx)
            .await
            .map_err(db_err)?;
            if live > 0 {
                return Err(http_err(409, "catalog.not_empty"));
            }
        }
    }
    let row: Option<CatalogRow> = sqlx::query_as(
        "UPDATE core.catalog SET
            name        = COALESCE($3, name),
            description = COALESCE($4, description),
            settings    = COALESCE($5, settings),
            status      = COALESCE($6, status),
            retired_at  = CASE WHEN $6 = 'retired' THEN now()
                               WHEN $6 = 'active' THEN NULL
                               ELSE retired_at END
         WHERE tenant_id=$1 AND catalog_id=$2
         RETURNING catalog_id, public_id, alias, name, description,
                   status, created_at, retired_at",
    )
    .bind(ctx.tenant_id)
    .bind(cat.catalog_id)
    .bind(
        body.name
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty()),
    )
    .bind(body.description.as_deref())
    .bind(body.settings.as_ref())
    .bind(body.status.as_deref())
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?;
    row.ok_or_else(|| http_err(404, "catalog not found"))
}

#[put("/catalogs/<reference>", data = "<body>")]
pub async fn update_catalog(
    cap: CapabilityGuard,
    state: &State<AppState>,
    reference: &str,
    body: Json<UpdateCatalogBody>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let reference = reference.to_string();
    let body = body.into_inner();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "catalogs.update").await?;
            let row = update_catalog_row(&mut tx, &ctx, &reference, &body).await?;
            let (entities, types) = catalog_counts(&mut tx, ctx.tenant_id, row.0).await?;
            tx.commit().await.map_err(db_err)?;
            Ok(catalog_json(&row, entities, types))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[post("/catalogs/<reference>/retire")]
pub async fn retire_catalog(
    cap: CapabilityGuard,
    state: &State<AppState>,
    reference: &str,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let reference = reference.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "catalogs.retire").await?;
            let body = UpdateCatalogBody {
                name: None,
                description: None,
                settings: None,
                status: Some("retired".to_string()),
            };
            let row = update_catalog_row(&mut tx, &ctx, &reference, &body).await?;
            let (entities, types) = catalog_counts(&mut tx, ctx.tenant_id, row.0).await?;
            tx.commit().await.map_err(db_err)?;
            Ok(catalog_json(&row, entities, types))
        })
        .await
        .map_err(|e| to_status(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_catalogs,
        get_catalog,
        create_catalog,
        update_catalog,
        retire_catalog
    ]
}
