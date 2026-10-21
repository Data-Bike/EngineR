//! Stateful schema administration API used by the local/Docker deployment.
//!
//! The route contract mirrors docs/06 §5 and enforces the production-facing
//! invariants that the admin UI depends on: lifecycle transitions, validation
//! before impact analysis, impact analysis before publication, optimistic
//! concurrency, immutable published revisions, and mandatory audit reasons.

use crate::workspace_routes::CapabilityGuard;
use crate::AppState;
use blake2::digest::consts::U32;
use blake2::{Blake2b, Digest};
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use enginer_persistence::RlsContext;
use rocket::http::{Header, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::response::Responder;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, routes, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::postgres::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: Value,
}

fn envelope<T: Serialize>(data: T) -> ApiEnvelope<T> {
    ApiEnvelope {
        data,
        meta: json!({ "request_id": uuid::Uuid::now_v7().to_string() }),
    }
}

#[derive(Debug, Clone, Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorBody {
    code: String,
    message_key: String,
    request_id: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    detail: Option<String>,
}

#[derive(Debug)]
pub struct ApiFailure {
    status: Status,
    code: &'static str,
    message: String,
}

impl ApiFailure {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: Status::BadRequest,
            code: "bad_request",
            message: message.into(),
        }
    }
}

impl<'r> Responder<'r, 'static> for ApiFailure {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let detail = (self.message.trim() != crate::ApiError::default_message(self.status)
            && !self.message.trim().is_empty())
        .then(|| self.message.clone());
        let body = ErrorEnvelope {
            error: ErrorBody {
                code: self.code.to_string(),
                message_key: format!("errors.{}", self.code),
                request_id: uuid::Uuid::now_v7().to_string(),
                message: self.message,
                detail,
            },
        };
        let mut response = Json(body).respond_to(request)?;
        response.set_status(self.status);
        Ok(response)
    }
}

/// JSON response carrying the current optimistic-concurrency token.
pub struct Etagged<T: Serialize> {
    etag: String,
    status: Status,
    body: ApiEnvelope<T>,
}

impl<T: Serialize> Etagged<T> {
    fn new(etag: String, data: T) -> Self {
        Self {
            etag,
            status: Status::Ok,
            body: envelope(data),
        }
    }

    fn created(etag: String, data: T) -> Self {
        Self {
            etag,
            status: Status::Created,
            body: envelope(data),
        }
    }
}

impl<'r, T: Serialize> Responder<'r, 'static> for Etagged<T> {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'static> {
        let mut response = Json(self.body).respond_to(request)?;
        response.set_status(self.status);
        response.set_header(Header::new("ETag", self.etag));
        response.set_header(Header::new("Cache-Control", "private, no-store"));
        Ok(response)
    }
}

#[derive(Debug)]
pub struct IfMatch(Option<String>);

impl IfMatch {
    pub fn value(&self) -> Option<&str> {
        self.0.as_deref()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IfMatch {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(Self(
            request.headers().get_one("If-Match").map(str::to_string),
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaChangeKind {
    Add,
    Update,
    Retire,
    Remove,
    Rename,
    Reclassify,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaChange {
    pub kind: SchemaChangeKind,
    pub resource_kind: String,
    pub resource_id: String,
    pub summary: String,
    #[serde(default)]
    pub breaking: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationFinding {
    pub severity: String,
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub revision_id: String,
    pub valid: bool,
    pub checked_at: String,
    pub findings: Vec<ValidationFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactDetails {
    pub new_fields: u64,
    pub removed_fields: u64,
    pub changed_fields: u64,
    pub migration_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub revision_id: String,
    pub generated_at: String,
    pub compatibility: String,
    pub breaking: bool,
    pub affected_entities: u64,
    pub affected_statements: u64,
    pub details: ImpactDetails,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaRevisionRecord {
    pub revision_id: String,
    pub display_name: String,
    pub description: String,
    pub status: String,
    pub semantic_version: String,
    pub parent_revision_id: Option<String>,
    pub version: u64,
    pub etag: String,
    pub types: Vec<Value>,
    pub predicates: Vec<Value>,
    /// Registry of relation groups ({id, label, order}) shown as sections
    /// on the relations tab. Predicates reference a group id via their
    /// `group` attribute; entries live at record level so they survive
    /// the draft → publish lifecycle.
    #[serde(default)]
    pub relation_groups: Vec<Value>,
    pub changes: Vec<SchemaChange>,
    pub type_count: usize,
    pub predicate_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub published_at: Option<String>,
    pub retired_at: Option<String>,
    pub content_hash: Option<String>,
    pub validation: Option<ValidationReport>,
    pub impact: Option<ImpactReport>,
    pub publish_reason: Option<String>,
    pub retire_reason: Option<String>,
    /// Owning catalog alias — surfaced so clients can distinguish same-named
    /// revision streams across catalogs (docs/28 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog: Option<String>,
}

impl SchemaRevisionRecord {
    fn refresh_derived(&mut self) {
        self.type_count = self.types.len();
        self.predicate_count = self.predicates.len();
        self.etag = etag(&self.revision_id, self.version);
        self.updated_at = chrono::Utc::now().to_rfc3339();
    }

    fn touch(&mut self) {
        self.version += 1;
        self.refresh_derived();
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDraftBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub base_revision: Option<String>,
    pub semantic_version: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDraftBody {
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    pub semantic_version: String,
    // Option, not #[serde(default)]: an absent field keeps the existing
    // content, an explicit [] clears it. A plain Vec would wipe the
    // schema on any metadata-only update.
    pub types: Option<Vec<Value>>,
    pub predicates: Option<Vec<Value>>,
    pub relation_groups: Option<Vec<Value>>,
    pub changes: Option<Vec<SchemaChange>>,
}

#[derive(Debug, Deserialize)]
pub struct ReasonBody {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DraftTombstone {
    pub revision_id: String,
    pub deleted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrationStatus {
    pub revision_id: String,
    pub status: String,
    pub migration_required: bool,
    pub completed_steps: u64,
    pub total_steps: u64,
    pub issues: u64,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// PostgreSQL storage (meta.schema_revision). The full SchemaRevisionRecord is
// stored in `content` jsonb; the indexed columns mirror status/semver/parent.
// ---------------------------------------------------------------------------

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, ApiFailure> {
    state.db_executor.clone().ok_or_else(|| ApiFailure {
        status: Status::ServiceUnavailable,
        code: "service_unavailable",
        message: "База данных недоступна".to_string(),
    })
}

/// ApiFailure crossing the executor boundary: "SF|<http>|<code>|<message>".
fn sf(status: u16, code: &'static str, message: impl Into<String>) -> DbError {
    DbError::Other {
        message: format!("SF|{status}|{code}|{}", message.into()),
    }
}

fn failure(e: &DbError) -> ApiFailure {
    if let DbError::Other { message } = e {
        if let Some(rest) = message.strip_prefix("SF|") {
            let mut parts = rest.splitn(3, '|');
            let status = parts
                .next()
                .and_then(|s| s.parse::<u16>().ok())
                .and_then(Status::from_code)
                .unwrap_or(Status::InternalServerError);
            let code = parts.next().unwrap_or("internal_error");
            let msg = parts.next().unwrap_or("error").to_string();
            return ApiFailure {
                status,
                code: Box::leak(code.to_string().into_boxed_str()),
                message: msg,
            };
        }
    }
    tracing::error!(error = %e, "schema admin db error");
    ApiFailure {
        status: Status::InternalServerError,
        code: "internal_error",
        message: "Внутренняя ошибка сервера".to_string(),
    }
}

fn sf_err(status: Status, code: &'static str, message: impl Into<String>) -> DbError {
    sf(status.code, code, message)
}

async fn schema_tx<'e>(
    pool: &'e PgPool,
    ctx: &crate::authn::AuthContext,
) -> Result<Transaction<'e, Postgres>, DbError> {
    let mut tx = pool.begin().await.map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    RlsContext {
        tenant_id: enginer_domain::TenantId(ctx.tenant_id),
        principal_id: enginer_domain::PrincipalId(ctx.principal_id),
        purpose: "schema.admin".to_string(),
        policy_revision: None,
    }
    .install_tx(&mut tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(tx)
}

async fn load_records(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog: Option<i64>,
) -> Result<Vec<SchemaRevisionRecord>, DbError> {
    let rows: Vec<(serde_json::Value, Option<String>)> = sqlx::query_as(
        "SELECT r.content, c.alias FROM meta.schema_revision r
         LEFT JOIN core.catalog c ON c.tenant_id=r.tenant_id AND c.catalog_id=r.catalog_id
         WHERE r.tenant_id=$1 AND ($2::bigint IS NULL OR r.catalog_id=$2)",
    )
    .bind(tenant)
    .bind(catalog)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(rows
        .into_iter()
        .filter_map(|(content, alias)| {
            serde_json::from_value::<SchemaRevisionRecord>(content)
                .ok()
                .map(|mut record| {
                    // The row's catalog is authoritative; the copy injected
                    // into content at write time is the fallback.
                    if record.catalog.is_none() {
                        record.catalog = alias;
                    }
                    record
                })
        })
        .collect())
}

/// Internal catalog id of a schema revision row.
async fn revision_catalog(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    revision_id: i64,
) -> Result<Option<i64>, DbError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT catalog_id FROM meta.schema_revision WHERE tenant_id=$1 AND revision_id=$2",
    )
    .bind(tenant)
    .bind(revision_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(row.map(|r| r.0))
}

/// Catalog gate for schema routes (docs/28 §4): the caller must hold the
/// catalog grant for `op` on the catalog that owns the record. `None`
/// (pre-catalog rows / system records) passes.
async fn require_catalog_access(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &crate::authn::AuthContext,
    catalog_id: Option<i64>,
    op: crate::dataperm::Op,
) -> Result<(), DbError> {
    let Some(cid) = catalog_id else {
        return Ok(());
    };
    let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT public_id, alias FROM core.catalog WHERE tenant_id=$1 AND catalog_id=$2",
    )
    .bind(ctx.tenant_id)
    .bind(cid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    let Some((pid, alias)) = row else {
        return Ok(());
    };
    let perm = crate::dataperm::DataPerm::load(tx, ctx.tenant_id, ctx.principal_id).await?;
    if !perm.can_catalog(&[&pid.to_string(), &alias], op) {
        return Err(sf_err(
            Status::Forbidden,
            "forbidden",
            "Нет доступа к каталогу",
        ));
    }
    Ok(())
}

/// Dataperm keys `[public_id, alias]` for a catalog id (empty when the
/// catalog row is gone — callers then get deny-by-default).
async fn catalog_key_list(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog_id: i64,
) -> Result<Vec<String>, DbError> {
    let row: Option<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT public_id, alias FROM core.catalog WHERE tenant_id=$1 AND catalog_id=$2",
    )
    .bind(tenant)
    .bind(catalog_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(row.map(|(p, a)| vec![p.to_string(), a]).unwrap_or_default())
}

/// Resolve `?catalog=` for schema routes; absent → `general` (the default
/// backfill catalog), unknown → 422.
async fn schema_catalog(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &crate::authn::AuthContext,
    param: Option<&str>,
) -> Result<crate::entity_pg::CatalogRef, DbError> {
    let reference = param.filter(|p| !p.trim().is_empty()).unwrap_or("general");
    crate::entity_pg::resolve_catalog(tx, ctx.tenant_id, reference.trim())
        .await
        .map_err(|_| {
            sf_err(
                Status::UnprocessableEntity,
                "catalog.unknown",
                "Каталог не найден",
            )
        })
}

fn is_draft(record: &SchemaRevisionRecord) -> bool {
    matches!(
        record.status.as_str(),
        "draft" | "validating" | "impact_ready"
    )
}

fn db_state(record: &SchemaRevisionRecord) -> &'static str {
    if is_draft(record) {
        "draft"
    } else if record.status == "published" {
        "published"
    } else {
        "retired"
    }
}

async fn save_record(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog_id: i64,
    actor: uuid::Uuid,
    record: &SchemaRevisionRecord,
) -> Result<(), DbError> {
    let revision_id: i64 = record
        .revision_id
        .parse()
        .map_err(|_| sf(500, "internal_error", "bad revision id"))?;
    let mut content = serde_json::to_value(record).map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    // The catalog alias rides inside content so consumers can render it
    // without a join; the authoritative ownership stays in the column.
    let catalog_row: Option<(String,)> =
        sqlx::query_as("SELECT alias FROM core.catalog WHERE tenant_id=$1 AND catalog_id=$2")
            .bind(tenant)
            .bind(catalog_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
    if let Some((alias,)) = catalog_row {
        if let Some(o) = content.as_object_mut() {
            o.insert("catalog".to_string(), Value::String(alias));
        }
    }
    let published_at = record
        .published_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    let retired_at = record
        .retired_at
        .as_deref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok());
    let parent: Option<i64> = record
        .parent_revision_id
        .as_deref()
        .and_then(|s| s.parse().ok());
    // Optimistic concurrency: every mutation path bumps `record.version`
    // (touch()), so the DO UPDATE only applies when the stored version is
    // strictly older. A stale in-flight writer loses instead of silently
    // overwriting a concurrent If-Match-passed change.
    let changed = sqlx::query(
        "INSERT INTO meta.schema_revision
         (tenant_id, revision_id, state, parent_revision_id, semantic_version, content,
          created_by, published_by, published_at, retired_at, catalog_id)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
         ON CONFLICT (tenant_id, revision_id) DO UPDATE
         SET state=EXCLUDED.state, parent_revision_id=EXCLUDED.parent_revision_id,
             semantic_version=EXCLUDED.semantic_version, content=EXCLUDED.content,
             published_by=EXCLUDED.published_by, published_at=EXCLUDED.published_at,
             retired_at=EXCLUDED.retired_at
         WHERE (meta.schema_revision.content->>'version')::bigint
               < (EXCLUDED.content->>'version')::bigint",
    )
    .bind(tenant)
    .bind(revision_id)
    .bind(db_state(record))
    .bind(parent)
    .bind(&record.semantic_version)
    .bind(content)
    .bind(actor)
    .bind(actor)
    .bind(published_at.map(|t| t.with_timezone(&chrono::Utc)))
    .bind(retired_at.map(|t| t.with_timezone(&chrono::Utc)))
    .bind(catalog_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?
    .rows_affected();
    if changed == 0 {
        return Err(sf_err(
            Status::Conflict,
            "conflict",
            "Схема была изменена параллельным запросом. Обновите данные и повторите.",
        ));
    }
    Ok(())
}

async fn delete_record(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    revision_id: i64,
) -> Result<(), DbError> {
    sqlx::query("DELETE FROM meta.schema_revision WHERE tenant_id=$1 AND revision_id=$2")
        .bind(tenant)
        .bind(revision_id)
        .execute(&mut **tx)
        .await
        .map_err(|e| DbError::Other {
            message: e.to_string(),
        })?;
    Ok(())
}

/// When a revision is published, mirror its type/predicate definitions into
/// the registry tables so the entity layer resolves aliases immediately.
/// Aliases are per-catalog (docs/28 §3): the same name in another catalog
/// keeps its own stable id.
async fn apply_definitions(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog_id: i64,
    revision_id: i64,
    record: &SchemaRevisionRecord,
) -> Result<(), DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };
    for t in &record.types {
        let alias = alias_of(t).unwrap_or_default().to_string();
        if alias.is_empty() {
            continue;
        }
        let class = t
            .get("entity_class")
            .and_then(Value::as_str)
            .unwrap_or("data");
        let class_i: i16 = match class {
            "schema" => 1,
            "policy" => 2,
            "workflow" => 3,
            "system" => 4,
            _ => 0,
        };
        // Keep the stable uuid across publishes: reuse if the alias exists
        // *inside this catalog* — an identical alias in another catalog is
        // a different type with its own type_id.
        let existing: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT type_id FROM meta.type_definition
             WHERE tenant_id=$1 AND stable_alias=$2 AND catalog_id=$3",
        )
        .bind(tenant)
        .bind(&alias)
        .bind(catalog_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let type_uuid = t
            .get("type_id")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<uuid::Uuid>().ok())
            .or(existing.map(|e| e.0))
            .unwrap_or_else(uuid::Uuid::now_v7);
        // A type returning after removal un-retires its registry row.
        sqlx::query(
            "UPDATE meta.type_definition SET retired_revision=NULL
             WHERE tenant_id=$1 AND type_id=$2",
        )
        .bind(tenant)
        .bind(type_uuid)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO meta.type_definition
             (tenant_id, type_id, stable_alias, entity_class, created_revision, catalog_id)
             VALUES ($1,$2,$3,$4,$5,$6)
             ON CONFLICT (tenant_id, type_id) DO NOTHING",
        )
        .bind(tenant)
        .bind(type_uuid)
        .bind(&alias)
        .bind(class_i)
        .bind(revision_id)
        .bind(catalog_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        if let Some(label) = t
            .get("label")
            .or_else(|| t.get("display_name"))
            .and_then(Value::as_str)
        {
            sqlx::query(
                "INSERT INTO meta.localized_text
                 (tenant_id, resource_kind, resource_id, field, locale, text, schema_revision)
                 VALUES ($1,'type',$2,'label','ru',$3,$4)
                 ON CONFLICT DO NOTHING",
            )
            .bind(tenant)
            .bind(type_uuid)
            .bind(label)
            .bind(revision_id)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
    }
    for p in &record.predicates {
        let alias = alias_of(p).unwrap_or_default().to_string();
        if alias.is_empty() {
            continue;
        }
        let existing: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT predicate_id FROM meta.predicate_definition
             WHERE tenant_id=$1 AND stable_alias=$2 AND catalog_id=$3",
        )
        .bind(tenant)
        .bind(&alias)
        .bind(catalog_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let pred_uuid = p
            .get("predicate_id")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<uuid::Uuid>().ok())
            .or(existing.map(|e| e.0))
            .unwrap_or_else(uuid::Uuid::now_v7);
        sqlx::query(
            "UPDATE meta.predicate_definition SET retired_revision=NULL
             WHERE tenant_id=$1 AND predicate_id=$2",
        )
        .bind(tenant)
        .bind(pred_uuid)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO meta.predicate_definition
             (tenant_id, predicate_id, stable_alias, is_edge, scalar_kind, confidentiality, created_revision, catalog_id)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8)
             ON CONFLICT (tenant_id, predicate_id) DO NOTHING",
        )
        .bind(tenant)
        .bind(pred_uuid)
        .bind(&alias)
        .bind(p.get("is_edge").and_then(Value::as_bool).unwrap_or(false))
        .bind(p.get("scalar_kind").and_then(Value::as_str))
        .bind(
            p.get("confidentiality")
                .or_else(|| p.get("confidentiality_class"))
                .and_then(Value::as_str)
                .unwrap_or("internal"),
        )
        .bind(revision_id)
        .bind(catalog_id)
        .execute(&mut **tx)
        .await
        .map_err(db)?;
        if let Some(label) = p
            .get("label")
            .or_else(|| p.get("display_name"))
            .and_then(Value::as_str)
        {
            sqlx::query(
                "INSERT INTO meta.localized_text
                 (tenant_id, resource_kind, resource_id, field, locale, text, schema_revision)
                 VALUES ($1,'predicate',$2,'label','ru',$3,$4)
                 ON CONFLICT DO NOTHING",
            )
            .bind(tenant)
            .bind(pred_uuid)
            .bind(label)
            .bind(revision_id)
            .execute(&mut **tx)
            .await
            .map_err(db)?;
        }
    }
    // Publish-diff retirement: definitions this catalog owns that are
    // absent from the new revision are marked retired — dropping a type
    // or predicate from the schema actually removes it from resolution
    // (docs/28 §3: namespaces are catalog-local). Entities of a retired
    // type remain readable (read paths join by type_id, not alias).
    let taliases: Vec<String> = record
        .types
        .iter()
        .filter_map(|t| alias_of(t).map(str::to_string))
        .collect();
    sqlx::query(
        "UPDATE meta.type_definition SET retired_revision=$3
         WHERE tenant_id=$1 AND catalog_id=$2 AND retired_revision IS NULL
           AND NOT (stable_alias = ANY($4))",
    )
    .bind(tenant)
    .bind(catalog_id)
    .bind(revision_id)
    .bind(&taliases)
    .execute(&mut **tx)
    .await
    .map_err(db)?;
    let paliases: Vec<String> = record
        .predicates
        .iter()
        .filter_map(|p| alias_of(p).map(str::to_string))
        .collect();
    sqlx::query(
        "UPDATE meta.predicate_definition SET retired_revision=$3
         WHERE tenant_id=$1 AND catalog_id=$2 AND retired_revision IS NULL
           AND NOT (stable_alias = ANY($4))",
    )
    .bind(tenant)
    .bind(catalog_id)
    .bind(revision_id)
    .bind(&paliases)
    .execute(&mut **tx)
    .await
    .map_err(db)?;

    // Derive identity policies from `identity`/`participates_in_identity`
    // field flags. Without this a published custom type declares identity
    // fields but dedup/identity_mode silently never applies — previously
    // only the seed created policies (for the built-in types).
    for t in &record.types {
        let talias = alias_of(t).unwrap_or_default().to_string();
        if talias.is_empty() {
            continue;
        }
        let type_row: Option<(uuid::Uuid,)> = sqlx::query_as(
            "SELECT type_id FROM meta.type_definition
             WHERE tenant_id=$1 AND stable_alias=$2 AND catalog_id=$3",
        )
        .bind(tenant)
        .bind(&talias)
        .bind(catalog_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db)?;
        let Some((type_uuid,)) = type_row else {
            continue;
        };
        let Some(fields) = t
            .get("fields")
            .or_else(|| t.get("predicates"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for f in fields {
            let is_identity = f
                .get("identity")
                .or_else(|| f.get("participates_in_identity"))
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if !is_identity {
                continue;
            }
            let key = f
                .get("predicate_id")
                .or_else(|| f.get("alias"))
                .or_else(|| f.get("stable_alias"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if key.is_empty() {
                continue;
            }
            let pred_uuid: uuid::Uuid = match key.parse::<uuid::Uuid>() {
                Ok(u) => u,
                Err(_) => {
                    // Same catalog-first fallback as entity writes: the
                    // catalog's own predicate wins, shared predicates
                    // resolve tenant-wide deterministically (docs/28 §5).
                    let row: Option<(uuid::Uuid,)> = sqlx::query_as(
                        "SELECT predicate_id FROM meta.predicate_definition
                         WHERE tenant_id=$1 AND stable_alias=$2
                         ORDER BY (catalog_id = $3) DESC,
                                  catalog_id, predicate_id
                         LIMIT 1",
                    )
                    .bind(tenant)
                    .bind(key)
                    .bind(catalog_id)
                    .fetch_optional(&mut **tx)
                    .await
                    .map_err(db)?;
                    match row {
                        Some((u,)) => u,
                        None => continue,
                    }
                }
            };
            let exists: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT policy_id FROM meta.identity_policy
                 WHERE tenant_id=$1 AND type_id=$2
                   AND key_component_predicate_ids=$3",
            )
            .bind(tenant)
            .bind(type_uuid)
            .bind(vec![pred_uuid])
            .fetch_optional(&mut **tx)
            .await
            .map_err(db)?;
            if exists.is_none() {
                sqlx::query(
                    "INSERT INTO meta.identity_policy
                     (tenant_id, policy_id, type_id, scope,
                      key_component_predicate_ids, normalizer_version,
                      match_mode, collision_action, resurrection_policy)
                     VALUES ($1,$2,$3,'global',$4,'v1','exact',
                             'return_existing','reuse')",
                )
                .bind(tenant)
                .bind(uuid::Uuid::now_v7())
                .bind(type_uuid)
                .bind(vec![pred_uuid])
                .execute(&mut **tx)
                .await
                .map_err(db)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
fn seed_revision() -> SchemaRevisionRecord {
    let timestamp = "2026-08-03T10:00:00Z".to_string();
    let types = vec![
        json!({ "type_id": "person", "stable_alias": "person", "entity_class": "data" }),
        json!({ "type_id": "legal_entity", "stable_alias": "legal_entity", "entity_class": "data" }),
        json!({ "type_id": "request", "stable_alias": "request", "entity_class": "workflow" }),
    ];
    let predicates = vec![
        json!({ "predicate_id": "display_name", "stable_alias": "display_name", "is_edge": false, "scalar_kind": "text", "min_cardinality": 1, "max_cardinality": 1 }),
        json!({ "predicate_id": "inn", "stable_alias": "inn", "is_edge": false, "scalar_kind": "text", "min_cardinality": 0, "max_cardinality": 1 }),
        json!({ "predicate_id": "related_to", "stable_alias": "related_to", "is_edge": true, "direction": "directed", "min_cardinality": 0 }),
    ];
    let mut record = SchemaRevisionRecord {
        revision_id: "1".to_string(),
        display_name: "SKR Core".to_string(),
        description: "Базовая опубликованная схема предметной области".to_string(),
        status: "published".to_string(),
        semantic_version: "1.0.0".to_string(),
        parent_revision_id: None,
        version: 1,
        etag: String::new(),
        type_count: types.len(),
        predicate_count: predicates.len(),
        types,
        predicates,
        relation_groups: Vec::new(),
        changes: Vec::new(),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
        published_at: Some(timestamp),
        retired_at: None,
        content_hash: None,
        validation: None,
        impact: None,
        publish_reason: Some("Initial SKR Core publication".to_string()),
        retire_reason: None,
        catalog: None,
    };
    record.refresh_derived();
    record.content_hash = Some(content_hash(&record));
    record
}

fn etag(id: &str, version: u64) -> String {
    format!("\"schema:{id}:{version}\"")
}

fn validate_reason(reason: &str) -> Result<(), ApiFailure> {
    if reason.trim().chars().count() < 10 {
        return Err(ApiFailure::bad_request(
            "Причина должна содержать не менее 10 символов.",
        ));
    }
    Ok(())
}

fn alias_of(value: &Value) -> Option<&str> {
    value
        .get("stable_alias")
        .or_else(|| value.get("alias"))
        .and_then(Value::as_str)
}

fn valid_alias(alias: &str) -> bool {
    let mut chars = alias.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

fn parse_semantic_version(value: &str) -> Option<(u64, u64, u64)> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
        })
    {
        return None;
    }
    Some((
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    ))
}

fn next_semantic_version(value: Option<&str>) -> String {
    let Some((major, minor, _)) = value.and_then(parse_semantic_version) else {
        return "1.0.0".to_string();
    };
    match minor.checked_add(1) {
        Some(next_minor) => format!("{major}.{next_minor}.0"),
        None => major.checked_add(1).map_or_else(
            || "1.0.0".to_string(),
            |next_major| format!("{next_major}.0.0"),
        ),
    }
}

fn validate_record(record: &SchemaRevisionRecord) -> ValidationReport {
    let mut findings = Vec::new();
    if record.display_name.trim().is_empty() {
        findings.push(ValidationFinding {
            severity: "error".to_string(),
            code: "display_name.required".to_string(),
            message: "Название ревизии обязательно".to_string(),
            path: Some("display_name".to_string()),
        });
    }
    if parse_semantic_version(&record.semantic_version).is_none() {
        findings.push(ValidationFinding {
            severity: "error".to_string(),
            code: "semantic_version.invalid".to_string(),
            message: "Версия должна соответствовать формату MAJOR.MINOR.PATCH".to_string(),
            path: Some("semantic_version".to_string()),
        });
    }

    for (collection_name, values) in [("types", &record.types), ("predicates", &record.predicates)]
    {
        let mut aliases = HashSet::new();
        for (index, value) in values.iter().enumerate() {
            match alias_of(value) {
                Some(alias) if !valid_alias(alias) => findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: "alias.invalid".to_string(),
                    message: format!("Некорректный alias `{alias}`"),
                    path: Some(format!("{collection_name}[{index}].stable_alias")),
                }),
                Some(alias) if !aliases.insert(alias.to_string()) => {
                    findings.push(ValidationFinding {
                        severity: "error".to_string(),
                        code: "alias.duplicate".to_string(),
                        message: format!("Alias `{alias}` используется повторно"),
                        path: Some(format!("{collection_name}[{index}].stable_alias")),
                    })
                }
                None => findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: "alias.required".to_string(),
                    message: "Для элемента не указан stable_alias".to_string(),
                    path: Some(format!("{collection_name}[{index}]")),
                }),
                _ => {}
            }
        }
    }

    let type_aliases: HashSet<&str> = record
        .types
        .iter()
        .filter_map(|t| {
            t.get("type_id")
                .or_else(|| t.get("stable_alias"))
                .and_then(Value::as_str)
        })
        .collect();
    let predicate_aliases: HashSet<&str> = record
        .predicates
        .iter()
        .filter_map(|p| {
            p.get("stable_alias")
                .or_else(|| p.get("predicate_id"))
                .and_then(Value::as_str)
        })
        .collect();

    // Field kinds declared on types must be known (docs/03 §7, docs/16 §6).
    for (t_index, type_def) in record.types.iter().enumerate() {
        let Some(fields) = type_def
            .get("fields")
            .or_else(|| type_def.get("predicates"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        for (f_index, field) in fields.iter().enumerate() {
            let path = format!("types[{t_index}].fields[{f_index}].kind");
            let Some(kind) = field
                .get("kind")
                .or_else(|| field.get("scalar_kind"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            if !crate::field_kinds::KNOWN_FIELD_KINDS.contains(&kind) {
                findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: "field.kind.unknown".to_string(),
                    message: format!("Неизвестный вид поля `{kind}`"),
                    path: Some(path),
                });
                continue;
            }
            if matches!(kind, "enum" | "dictionary")
                && field
                    .get("values")
                    .or_else(|| field.get("options"))
                    .and_then(Value::as_array)
                    .map_or(true, |vs| vs.is_empty())
            {
                findings.push(ValidationFinding {
                    severity: "warning".to_string(),
                    code: "field.values.recommended".to_string(),
                    message: "Для enum/dictionary рекомендуется список `values`".to_string(),
                    path: Some(path.clone()),
                });
            }
            if kind == "entity_reference" {
                let target = field
                    .get("target_type_id")
                    .or_else(|| field.get("range"))
                    .and_then(Value::as_str);
                match target {
                    None => findings.push(ValidationFinding {
                        severity: "warning".to_string(),
                        code: "field.target_type.recommended".to_string(),
                        message: "Для entity_reference рекомендуется `target_type_id`".to_string(),
                        path: Some(path),
                    }),
                    Some(t) if !type_aliases.contains(t) => findings.push(ValidationFinding {
                        severity: "error".to_string(),
                        code: "field.target_type.unknown".to_string(),
                        message: format!("entity_reference указывает на неизвестный тип `{t}`"),
                        path: Some(path),
                    }),
                    _ => {}
                }
            }
        }
    }

    // Referential integrity (docs/28 §4, mirror of bundle::validate_references):
    // type fields must name a declared predicate; edge predicates must point
    // domain/range at declared types.
    for (t_index, type_def) in record.types.iter().enumerate() {
        let fields = type_def
            .get("fields")
            .or_else(|| type_def.get("predicates"))
            .and_then(Value::as_array);
        let Some(fields) = fields else { continue };
        for (f_index, field) in fields.iter().enumerate() {
            let Some(alias) = field
                .get("alias")
                .or_else(|| field.get("predicate_id"))
                .or_else(|| field.get("predicate"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            if !predicate_aliases.contains(alias) {
                findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: "field.predicate.unknown".to_string(),
                    message: format!("Поле ссылается на необъявленный предикат `{alias}`"),
                    path: Some(format!("types[{t_index}].fields[{f_index}].alias")),
                });
            }
        }
    }
    let group_ids: HashSet<&str> = record
        .relation_groups
        .iter()
        .filter_map(|g| g.get("id").and_then(Value::as_str))
        .collect();
    for (p_index, predicate) in record.predicates.iter().enumerate() {
        let is_edge = predicate
            .get("is_edge")
            .and_then(Value::as_bool)
            .unwrap_or(false)
            || predicate.get("domain").is_some()
            || predicate.get("range").is_some();
        let p_alias = alias_of(predicate).unwrap_or("?");
        // `group` must resolve to a declared relation_group id, otherwise the
        // relation lands in an unnamed bucket in the UI.
        if let Some(group) = predicate.get("group").and_then(Value::as_str) {
            if !group.is_empty() && !group_ids.contains(group) {
                findings.push(ValidationFinding {
                    severity: "warning".to_string(),
                    code: "edge.group.unknown".to_string(),
                    message: format!(
                        "Предикат `{p_alias}` ссылается на необъявленную группу `{group}`"
                    ),
                    path: Some(format!("predicates[{p_index}].group")),
                });
            }
        }
        if !is_edge {
            continue;
        }
        for (key, code) in [("domain", "edge.domain.unknown"), ("range", "edge.range.unknown")] {
            match predicate.get(key).and_then(Value::as_str) {
                Some(t) if !type_aliases.contains(t) => findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: code.to_string(),
                    message: format!("Ребро `{p_alias}` ссылается на неизвестный тип `{t}` ({key})"),
                    path: Some(format!("predicates[{p_index}].{key}")),
                }),
                None => findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: format!("edge.{key}.required"),
                    message: format!("У ребра `{p_alias}` не задан {key}"),
                    path: Some(format!("predicates[{p_index}].{key}")),
                }),
                _ => {}
            }
        }
    }

    if record.types.is_empty() {
        findings.push(ValidationFinding {
            severity: "warning".to_string(),
            code: "types.empty".to_string(),
            message: "В ревизии нет определений типов".to_string(),
            path: Some("types".to_string()),
        });
    }
    if record.changes.is_empty() {
        findings.push(ValidationFinding {
            severity: "warning".to_string(),
            code: "changes.empty".to_string(),
            message: "Не заполнен журнал изменений".to_string(),
            path: Some("changes".to_string()),
        });
    }

    for (index, change) in record.changes.iter().enumerate() {
        for (field, value) in [
            ("resource_kind", change.resource_kind.as_str()),
            ("resource_id", change.resource_id.as_str()),
            ("summary", change.summary.as_str()),
        ] {
            if value.trim().is_empty() {
                findings.push(ValidationFinding {
                    severity: "error".to_string(),
                    code: format!("change.{field}.required"),
                    message: format!("Поле `{field}` в журнале изменений обязательно"),
                    path: Some(format!("changes[{index}].{field}")),
                });
            }
        }
    }

    ValidationReport {
        revision_id: record.revision_id.clone(),
        valid: !findings.iter().any(|finding| finding.severity == "error"),
        checked_at: chrono::Utc::now().to_rfc3339(),
        findings,
    }
}

fn definition_id(value: &Value) -> Option<&str> {
    value
        .get("type_id")
        .or_else(|| value.get("predicate_id"))
        .or_else(|| value.get("stable_id"))
        .and_then(Value::as_str)
        .or_else(|| alias_of(value))
}

fn definition_map(values: &[Value]) -> HashMap<String, String> {
    values
        .iter()
        .filter_map(|value| definition_id(value).map(|id| (id.to_string(), canonical_json(value))))
        .collect()
}

fn definition_diff(before: &[Value], after: &[Value]) -> (u64, u64, u64) {
    let before = definition_map(before);
    let after = definition_map(after);
    let added = after.keys().filter(|id| !before.contains_key(*id)).count() as u64;
    let removed = before.keys().filter(|id| !after.contains_key(*id)).count() as u64;
    let changed = after
        .iter()
        .filter(|(id, value)| before.get(*id).is_some_and(|previous| previous != *value))
        .count() as u64;
    (added, removed, changed)
}

fn impact_record(
    record: &SchemaRevisionRecord,
    parent: Option<&SchemaRevisionRecord>,
) -> ImpactReport {
    let (type_added, type_removed, type_changed) = definition_diff(
        parent.map_or(&[], |value| value.types.as_slice()),
        &record.types,
    );
    let (predicate_added, predicate_removed, predicate_changed) = definition_diff(
        parent.map_or(&[], |value| value.predicates.as_slice()),
        &record.predicates,
    );
    let actual_added = type_added + predicate_added;
    let actual_removed = type_removed + predicate_removed;
    let actual_changed = type_changed + predicate_changed;
    let mut logged_added = 0;
    let mut logged_removed = 0;
    let mut logged_changed = 0;
    let mut breaking = false;
    for change in &record.changes {
        match change.kind {
            SchemaChangeKind::Add => logged_added += 1,
            SchemaChangeKind::Remove | SchemaChangeKind::Retire => logged_removed += 1,
            SchemaChangeKind::Update | SchemaChangeKind::Rename | SchemaChangeKind::Reclassify => {
                logged_changed += 1;
            }
        }
        breaking |= change.breaking
            || matches!(
                change.kind,
                SchemaChangeKind::Remove | SchemaChangeKind::Retire
            );
    }
    let added = actual_added.max(logged_added);
    let removed = actual_removed.max(logged_removed);
    let changed = actual_changed.max(logged_changed);
    breaking |= actual_removed > 0;
    let migration_required = breaking || removed > 0 || changed > 0;
    let compatibility = if breaking {
        "breaking"
    } else if migration_required {
        "conditionally_compatible"
    } else if added > 0 {
        "additive"
    } else {
        "presentation_only"
    };
    ImpactReport {
        revision_id: record.revision_id.clone(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        compatibility: compatibility.to_string(),
        breaking,
        affected_entities: (removed + changed) * 128,
        affected_statements: (removed + changed) * 512,
        details: ImpactDetails {
            new_fields: added,
            removed_fields: removed,
            changed_fields: changed,
            migration_required,
        },
        blockers: Vec::new(),
    }
}

fn canonical_json(value: &Value) -> String {
    fn canonicalize(value: &Value) -> Value {
        match value {
            Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
            Value::Object(values) => {
                let mut entries = values.iter().collect::<Vec<_>>();
                entries.sort_by(|(left, _), (right, _)| left.cmp(right));
                Value::Object(
                    entries
                        .into_iter()
                        .map(|(key, value)| (key.clone(), canonicalize(value)))
                        .collect(),
                )
            }
            _ => value.clone(),
        }
    }
    serde_json::to_string(&canonicalize(value)).unwrap_or_default()
}

fn content_hash(record: &SchemaRevisionRecord) -> String {
    let mut types = record.types.iter().map(canonical_json).collect::<Vec<_>>();
    let mut predicates = record
        .predicates
        .iter()
        .map(canonical_json)
        .collect::<Vec<_>>();
    types.sort();
    predicates.sort();
    let mut hasher = Blake2b::<U32>::new();
    hasher.update(types.join("\u{1f}").as_bytes());
    hasher.update(predicates.join("\u{1f}").as_bytes());
    hex::encode(hasher.finalize())
}

#[get("/schemas/drafts?<catalog>")]
pub async fn list_drafts(
    cap: CapabilityGuard,
    state: &State<AppState>,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let cat =
                crate::entity_pg::optional_catalog(&mut tx, ctx.tenant_id, catalog.as_deref())
                    .await
                    .map_err(|_| {
                        sf_err(
                            Status::UnprocessableEntity,
                            "catalog.unknown",
                            "Каталог не найден",
                        )
                    })?;
            if let Some(c) = &cat {
                require_catalog_access(
                    &mut tx,
                    &ctx,
                    Some(c.catalog_id),
                    crate::dataperm::Op::Read,
                )
                .await?;
            }
            // Drafts of catalogs the principal cannot read are hidden —
            // the catalog gate applies per record, not only to `?catalog`.
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id).await?;
            let rows: Vec<(i64, serde_json::Value, Option<String>)> = sqlx::query_as(
                "SELECT r.catalog_id, r.content, c.alias FROM meta.schema_revision r
                 LEFT JOIN core.catalog c ON c.tenant_id=r.tenant_id AND c.catalog_id=r.catalog_id
                 WHERE r.tenant_id=$1 AND ($2::bigint IS NULL OR r.catalog_id=$2)",
            )
            .bind(ctx.tenant_id)
            .bind(cat.map(|c| c.catalog_id))
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            let mut drafts: Vec<SchemaRevisionRecord> = Vec::new();
            for (cid, content, alias) in rows {
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    continue;
                }
                if let Ok(mut record) = serde_json::from_value::<SchemaRevisionRecord>(content) {
                    if record.catalog.is_none() {
                        record.catalog = alias;
                    }
                    if is_draft(&record) {
                        drafts.push(record);
                    }
                }
            }
            drafts.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            Ok(serde_json::to_value(drafts).unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

async fn find_record(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    id: &str,
    drafts_only: bool,
) -> Result<Option<SchemaRevisionRecord>, DbError> {
    Ok(load_records(tx, tenant, None)
        .await?
        .into_iter()
        .find(|r| r.revision_id == id && (!drafts_only || is_draft(r))))
}

/// Catalog gate for a record addressed by revision id: 403 when the
/// principal lacks the catalog grant for `op`.
async fn gate_record_catalog(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &crate::authn::AuthContext,
    revision_id: &str,
    op: crate::dataperm::Op,
) -> Result<i64, DbError> {
    let rid: i64 = revision_id
        .parse()
        .map_err(|_| sf(500, "internal_error", "bad revision id"))?;
    let cid = revision_catalog(tx, ctx.tenant_id, rid).await?;
    require_catalog_access(tx, ctx, cid, op).await?;
    Ok(cid.unwrap_or(0))
}

#[get("/schemas/drafts/<id>")]
pub async fn get_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let record = find_record(&mut tx, ctx.tenant_id, &id, true)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
            gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Read).await?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

/// `POST /schemas/drafts` — `?catalog=` picks which catalog's schema stream
/// to draft against (default `general`, docs/28 §6).
#[post("/schemas/drafts?<catalog>", data = "<body>")]
pub async fn create_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    catalog: Option<String>,
    body: Json<CreateDraftBody>,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let body = body.into_inner();
    if body.name.trim().is_empty() {
        return Err(ApiFailure::bad_request("Название черновика обязательно"));
    }
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            // Serialize revision-id allocation per tenant: next_id is
            // max(revision_id)+1, so concurrent creates would collide.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 7919))")
                .bind(ctx.tenant_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
            // Every catalog owns an independent draft→publish stream —
            // drafts are created inside the resolved catalog only. A
            // retired catalog takes no new schema work.
            let cat = schema_catalog(&mut tx, &ctx, catalog.as_deref()).await?;
            if cat.status != "active" {
                return Err(sf_err(
                    Status::Conflict,
                    "catalog.retired",
                    "Каталог погашен — изменение схемы недоступно",
                ));
            }
            require_catalog_access(
                &mut tx,
                &ctx,
                Some(cat.catalog_id),
                crate::dataperm::Op::Update,
            )
            .await?;
            let all = load_records(&mut tx, ctx.tenant_id, Some(cat.catalog_id)).await?;
            let revisions: Vec<&SchemaRevisionRecord> =
                all.iter().filter(|r| !is_draft(r)).collect();
            let base = match body.base_revision.as_deref() {
                Some(bid) => Some(
                    revisions
                        .iter()
                        .find(|r| r.revision_id == bid)
                        .copied()
                        .cloned()
                        .ok_or_else(|| {
                            sf_err(
                                Status::BadRequest,
                                "bad_request",
                                "Базовая ревизия не найдена",
                            )
                        })?,
                ),
                // No explicit base → inherit from the latest published
                // revision, otherwise the draft starts from an empty schema
                // and publishing it would wipe the catalog.
                None => revisions
                    .iter()
                    .filter(|r| r.status == "published")
                    .max_by_key(|r| r.revision_id.parse::<i64>().unwrap_or(0))
                    .copied()
                    .cloned(),
            };
            let latest_version = revisions
                .iter()
                .filter_map(|r| {
                    parse_semantic_version(&r.semantic_version)
                        .map(|ver| (ver, r.semantic_version.as_str()))
                })
                .max_by_key(|(ver, _)| *ver)
                .map(|(_, value)| value);
            let semantic_version = body
                .semantic_version
                .unwrap_or_else(|| next_semantic_version(latest_version));
            // revision_id stays unique tenant-wide (PK tenant+revision_id)
            // even though each catalog has its own revision stream.
            let (next_id,): (i64,) = sqlx::query_as(
                "SELECT COALESCE(MAX(revision_id),0)+1 FROM meta.schema_revision
                 WHERE tenant_id=$1",
            )
            .bind(ctx.tenant_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            // Inheritance is namespace-local: a base revision authored
            // before catalogs may list definitions that now live in other
            // catalogs' registries. Drafts in this catalog keep only the
            // entries this catalog owns (or that no registry claims), so
            // publishing cannot clone `person`/`name` into a second
            // namespace (docs/28 §3).
            let (owned_types, owned_preds, owned_groups) = if base.is_some() {
                let trows: Vec<(String, Option<i64>)> = sqlx::query_as(
                    "SELECT stable_alias, catalog_id FROM meta.type_definition WHERE tenant_id=$1",
                )
                .bind(ctx.tenant_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                let prows: Vec<(String, Option<i64>)> = sqlx::query_as(
                    "SELECT stable_alias, catalog_id FROM meta.predicate_definition WHERE tenant_id=$1",
                )
                .bind(ctx.tenant_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                let keep = |rows: &[(String, Option<i64>)], v: &Value| -> bool {
                    let alias = alias_of(v).unwrap_or_default();
                    if alias.is_empty() {
                        return true;
                    }
                    match rows.iter().find(|(a, _)| a == alias) {
                        Some((_, cid)) => *cid == Some(cat.catalog_id),
                        None => true,
                    }
                };
                let b = base.as_ref().unwrap();
                (
                    b.types.iter().filter(|v| keep(&trows, v)).cloned().collect::<Vec<_>>(),
                    b.predicates.iter().filter(|v| keep(&prows, v)).cloned().collect::<Vec<_>>(),
                    b.relation_groups.iter().filter(|v| keep(&prows, v)).cloned().collect::<Vec<_>>(),
                )
            } else {
                (Vec::new(), Vec::new(), Vec::new())
            };
            let now = chrono::Utc::now().to_rfc3339();
            let mut record = SchemaRevisionRecord {
                revision_id: next_id.to_string(),
                display_name: body.name.trim().to_string(),
                description: body.description.trim().to_string(),
                status: "draft".to_string(),
                semantic_version,
                parent_revision_id: base.as_ref().map(|r| r.revision_id.clone()),
                version: 1,
                etag: String::new(),
                types: owned_types,
                predicates: owned_preds,
                relation_groups: owned_groups,
                changes: Vec::new(),
                type_count: 0,
                predicate_count: 0,
                created_at: now.clone(),
                updated_at: now,
                published_at: None,
                retired_at: None,
                content_hash: None,
                validation: None,
                impact: None,
                publish_reason: None,
                retire_reason: None,
                catalog: Some(cat.alias.clone()),
            };
            record.refresh_derived();
            save_record(&mut tx, ctx.tenant_id, cat.catalog_id, ctx.principal_id, &record)
                .await?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::created(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[put("/schemas/drafts/<id>", data = "<body>")]
pub async fn update_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
    if_match: IfMatch,
    body: Json<UpdateDraftBody>,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let if_match_s = if_match.0.clone();
    let body = body.into_inner();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let mut record = find_record(&mut tx, ctx.tenant_id, &id, true)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
            let cat_id =
                gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Update).await?;
            require_if_match_pg(&record, &if_match_s)?;
            if !matches!(
                record.status.as_str(),
                "draft" | "validating" | "impact_ready"
            ) {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Редактировать можно только неопубликованный черновик",
                ));
            }
            record.display_name = body.display_name.trim().to_string();
            record.description = body.description.trim().to_string();
            record.semantic_version = body.semantic_version.trim().to_string();
            if let Some(t) = body.types {
                record.types = t;
            }
            if let Some(p) = body.predicates {
                record.predicates = p;
            }
            if let Some(g) = body.relation_groups {
                record.relation_groups = g;
            }
            if let Some(c) = body.changes {
                record.changes = c;
            }
            record.status = "draft".to_string();
            record.validation = None;
            record.impact = None;
            record.touch();
            save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, &record).await?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[delete("/schemas/drafts/<id>")]
pub async fn delete_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
    if_match: IfMatch,
) -> Result<Json<ApiEnvelope<DraftTombstone>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let id2 = id.clone();
    let if_match_s = if_match.0.clone();
    exec.run(Deadline::in_secs(10), move |pool| async move {
        let mut tx = schema_tx(&pool, &ctx).await?;
        let record = find_record(&mut tx, ctx.tenant_id, &id2, true)
            .await?
            .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
        gate_record_catalog(&mut tx, &ctx, &id2, crate::dataperm::Op::Update).await?;
        require_if_match_pg(&record, &if_match_s)?;
        let rid: i64 = id2
            .parse()
            .map_err(|_| sf(500, "internal_error", "bad revision id"))?;
        delete_record(&mut tx, ctx.tenant_id, rid).await?;
        tx.commit().await.map_err(|e| DbError::Other {
            message: e.to_string(),
        })?;
        Ok(Value::Null)
    })
    .await
    .map_err(|e| failure(&e))?;
    Ok(Json(envelope(DraftTombstone {
        revision_id: id,
        deleted: true,
    })))
}

#[post("/schemas/drafts/<id>/validate")]
pub async fn validate_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let mut record = find_record(&mut tx, ctx.tenant_id, &id, true)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
            let cat_id =
                gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Update).await?;
            if record.status != "draft" {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Повторная проверка требует новой draft-ревизии",
                ));
            }
            let report = validate_record(&record);
            record.status = if report.valid { "validating" } else { "draft" }.to_string();
            record.validation = Some(report.clone());
            record.impact = None;
            record.touch();
            save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, &record).await?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&report).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[post("/schemas/drafts/<id>/impact")]
pub async fn impact_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let cat_id =
                gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Update).await?;
            let all = load_records(&mut tx, ctx.tenant_id, Some(cat_id)).await?;
            let idx = all
                .iter()
                .position(|r| r.revision_id == id && is_draft(r))
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
            let mut record = all[idx].clone();
            if !record
                .validation
                .as_ref()
                .is_some_and(|report| report.valid)
            {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Перед анализом влияния выполните успешную валидацию",
                ));
            }
            let parent = record
                .parent_revision_id
                .as_deref()
                .and_then(|pid| all.iter().find(|r| r.revision_id == pid).cloned());
            let report = impact_record(&record, parent.as_ref());
            record.status = "impact_ready".to_string();
            record.impact = Some(report.clone());
            record.touch();
            save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, &record).await?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&report).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[post("/schemas/drafts/<id>/publish", data = "<body>")]
pub async fn publish_draft(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
    if_match: IfMatch,
    body: Json<ReasonBody>,
) -> Result<Etagged<Value>, ApiFailure> {
    validate_reason(&body.reason)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let if_match_s = if_match.0.clone();
    let reason = body.reason.trim().to_string();
    let v = exec
        .run(Deadline::in_secs(20), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            // Publish mutates *other* revisions (retires the published set) and
            // mirrors definitions into the registry — serialize publishes per
            // tenant so two concurrent drafts can't interleave.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 7919))")
                .bind(ctx.tenant_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
            // Publish is catalog-scoped: it retires only the published
            // revision of the same catalog and mirrors definitions into
            // that catalog's namespace (docs/28 §3).
            let cat_id =
                gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Update).await?;
            let mut all = load_records(&mut tx, ctx.tenant_id, Some(cat_id)).await?;
            let idx = all
                .iter()
                .position(|r| r.revision_id == id && is_draft(r))
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Черновик схемы не найден"))?;
            require_if_match_pg(&all[idx], &if_match_s)?;
            if all[idx].status != "impact_ready" {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Публикация доступна только после успешного анализа влияния",
                ));
            }
            if all[idx]
                .impact
                .as_ref()
                .is_some_and(|impact| !impact.blockers.is_empty())
            {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Публикация заблокирована результатами анализа влияния",
                ));
            }
            let candidate =
                parse_semantic_version(&all[idx].semantic_version).ok_or_else(|| {
                    sf_err(
                        Status::UnprocessableEntity,
                        "schema.state_invalid",
                        "Некорректная semantic version",
                    )
                })?;
            if all.iter().any(|r| {
                !is_draft(r)
                    && parse_semantic_version(&r.semantic_version).is_some_and(|v| v >= candidate)
            }) {
                return Err(sf_err(
                    Status::Conflict,
                    "conflict",
                    "Semantic version должна быть выше всех ранее опубликованных ревизий",
                ));
            }
            let retired_at = chrono::Utc::now().to_rfc3339();
            for rev in all.iter_mut().filter(|r| r.status == "published") {
                rev.status = "retired".to_string();
                rev.retired_at = Some(retired_at.clone());
                rev.retire_reason = Some("Superseded by a newer revision".to_string());
                rev.touch();
                save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, rev).await?;
            }
            let mut record = all[idx].clone();
            record.status = "published".to_string();
            record.published_at = Some(chrono::Utc::now().to_rfc3339());
            record.publish_reason = Some(reason);
            record.content_hash = Some(content_hash(&record));
            record.touch();
            let rid: i64 = record.revision_id.parse().unwrap_or(0);
            apply_definitions(&mut tx, ctx.tenant_id, cat_id, rid, &record).await?;
            save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, &record).await?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[get("/schemas/published?<catalog>")]
pub async fn list_revisions(
    cap: CapabilityGuard,
    state: &State<AppState>,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let cat =
                crate::entity_pg::optional_catalog(&mut tx, ctx.tenant_id, catalog.as_deref())
                    .await
                    .map_err(|_| {
                        sf_err(
                            Status::UnprocessableEntity,
                            "catalog.unknown",
                            "Каталог не найден",
                        )
                    })?;
            if let Some(c) = &cat {
                require_catalog_access(
                    &mut tx,
                    &ctx,
                    Some(c.catalog_id),
                    crate::dataperm::Op::Read,
                )
                .await?;
            }
            // Per-record catalog gate — same rule as list_drafts.
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id).await?;
            let rows: Vec<(i64, serde_json::Value, Option<String>)> = sqlx::query_as(
                "SELECT r.catalog_id, r.content, c.alias FROM meta.schema_revision r
                 LEFT JOIN core.catalog c ON c.tenant_id=r.tenant_id AND c.catalog_id=r.catalog_id
                 WHERE r.tenant_id=$1 AND ($2::bigint IS NULL OR r.catalog_id=$2)",
            )
            .bind(ctx.tenant_id)
            .bind(cat.map(|c| c.catalog_id))
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            let mut revisions: Vec<SchemaRevisionRecord> = Vec::new();
            for (cid, content, alias) in rows {
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    continue;
                }
                if let Ok(mut record) = serde_json::from_value::<SchemaRevisionRecord>(content) {
                    // Pre-catalog rows carry the stream only as a column —
                    // hydrate so clients can tell revisions apart per catalog.
                    if record.catalog.is_none() {
                        record.catalog = alias;
                    }
                    if !is_draft(&record) {
                        revisions.push(record);
                    }
                }
            }
            revisions.sort_by_key(|r| {
                std::cmp::Reverse(r.revision_id.parse::<u64>().unwrap_or_default())
            });
            Ok(serde_json::to_value(revisions).unwrap_or(Value::Null))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

#[get("/schemas/revisions/<id>")]
pub async fn get_revision(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
) -> Result<Etagged<Value>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let record = find_record(&mut tx, ctx.tenant_id, &id, false)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Ревизия схемы не найдена"))?;
            gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Read).await?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[post("/schemas/revisions/<id>/retire", data = "<body>")]
pub async fn retire_revision(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
    if_match: IfMatch,
    body: Json<ReasonBody>,
) -> Result<Etagged<Value>, ApiFailure> {
    validate_reason(&body.reason)?;
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let if_match_s = if_match.0.clone();
    let reason = body.reason.trim().to_string();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let mut record = find_record(&mut tx, ctx.tenant_id, &id, false)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Ревизия схемы не найдена"))?;
            let cat_id =
                gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Update).await?;
            require_if_match_pg(&record, &if_match_s)?;
            if record.status != "published" {
                return Err(sf_err(
                    Status::UnprocessableEntity,
                    "schema.state_invalid",
                    "Погасить можно только текущую опубликованную ревизию",
                ));
            }
            record.status = "retired".to_string();
            record.retired_at = Some(chrono::Utc::now().to_rfc3339());
            record.retire_reason = Some(reason);
            record.touch();
            save_record(&mut tx, ctx.tenant_id, cat_id, ctx.principal_id, &record).await?;
            // Retiring the published revision withdraws its registry
            // definitions — resolve_type/resolve_predicates only honour
            // rows with retired_revision IS NULL, so without this the
            // catalog's types would remain creatable with no published
            // schema behind them.
            let rev_num: i64 = record.revision_id.parse().map_err(|_| {
                sf_err(
                    Status::InternalServerError,
                    "internal_error",
                    "bad revision id",
                )
            })?;
            sqlx::query(
                "UPDATE meta.type_definition SET retired_revision=$3
                 WHERE tenant_id=$1 AND catalog_id=$2 AND retired_revision IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(cat_id)
            .bind(rev_num)
            .execute(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            sqlx::query(
                "UPDATE meta.predicate_definition SET retired_revision=$3
                 WHERE tenant_id=$1 AND catalog_id=$2 AND retired_revision IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(cat_id)
            .bind(rev_num)
            .execute(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            tx.commit().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            Ok(json!({"etag": record.etag, "data": serde_json::to_value(&record).unwrap()}))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Etagged::new(
        v["etag"].as_str().unwrap_or_default().to_string(),
        v["data"].clone(),
    ))
}

#[get("/schemas/revisions/<id>/migration-status")]
pub async fn migration_status(
    cap: CapabilityGuard,
    state: &State<AppState>,
    id: &str,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let id = id.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let record = find_record(&mut tx, ctx.tenant_id, &id, false)
                .await?
                .ok_or_else(|| sf_err(Status::NotFound, "not_found", "Ревизия схемы не найдена"))?;
            gate_record_catalog(&mut tx, &ctx, &id, crate::dataperm::Op::Read).await?;
            let required = record
                .impact
                .as_ref()
                .is_some_and(|impact| impact.details.migration_required);
            Ok(serde_json::to_value(MigrationStatus {
                revision_id: id,
                status: if required {
                    "completed".to_string()
                } else {
                    "not_required".to_string()
                },
                migration_required: required,
                completed_steps: u64::from(required),
                total_steps: u64::from(required),
                issues: 0,
                updated_at: record.updated_at.clone(),
            })
            .unwrap())
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

fn require_if_match_pg(
    record: &SchemaRevisionRecord,
    header: &Option<String>,
) -> Result<(), DbError> {
    match header {
        Some(value) if value == &record.etag => Ok(()),
        Some(_) => Err(sf_err(
            Status::PreconditionFailed,
            "precondition_failed",
            "Схема изменилась. Обновите страницу и повторите действие.",
        )),
        None => Err(sf_err(
            Status::PreconditionFailed,
            "precondition_failed",
            "Для изменения схемы требуется заголовок If-Match.",
        )),
    }
}

// --- Schema catalog (published revision content) ---

/// Load `(catalog_id, record)` pairs of published revisions — one per
/// catalog stream, or of a single catalog when `catalog` is set
/// (docs/28 §3).
async fn published_records(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    catalog: Option<i64>,
) -> Result<Vec<(i64, SchemaRevisionRecord)>, DbError> {
    let rows: Vec<(i64, serde_json::Value, Option<String>)> = sqlx::query_as(
        "SELECT r.catalog_id, r.content, c.alias FROM meta.schema_revision r
         LEFT JOIN core.catalog c ON c.tenant_id=r.tenant_id AND c.catalog_id=r.catalog_id
         WHERE r.tenant_id=$1 AND ($2::bigint IS NULL OR r.catalog_id=$2)",
    )
    .bind(tenant)
    .bind(catalog)
    .fetch_all(&mut **tx)
    .await
    .map_err(|e| DbError::Other {
        message: e.to_string(),
    })?;
    Ok(rows
        .into_iter()
        .filter_map(|(cid, content, alias)| {
            serde_json::from_value::<SchemaRevisionRecord>(content)
                .ok()
                .map(|mut record| {
                    // Rows written before the catalog field existed have it
                    // only as a column — hydrate so list responses can
                    // distinguish streams.
                    if record.catalog.is_none() {
                        record.catalog = alias;
                    }
                    (cid, record)
                })
        })
        .filter(|(_, r)| r.status == "published")
        .collect())
}

fn type_entry_to_record(
    entry: &Value,
    revision: i64,
    entity_count: usize,
) -> crate::workspace_routes::SchemaTypeRecord {
    let alias = alias_of(entry).unwrap_or_default().to_string();
    let fields: Vec<crate::workspace_routes::TypeFieldRecord> = entry
        .get("fields")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|f| crate::workspace_routes::TypeFieldRecord {
                    alias: f
                        .get("alias")
                        .or_else(|| f.get("predicate_id"))
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    label: f
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    kind: f
                        .get("kind")
                        .or_else(|| f.get("scalar_kind"))
                        .and_then(Value::as_str)
                        .unwrap_or("text")
                        .to_string(),
                    required: f.get("required").and_then(Value::as_bool).unwrap_or(false),
                    identity: f.get("identity").and_then(Value::as_bool).unwrap_or(false),
                    target_type_id: f
                        .get("target_type_id")
                        .or_else(|| f.get("range"))
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    values: f
                        .get("values")
                        .or_else(|| f.get("options"))
                        .and_then(Value::as_array)
                        .map(|vs| {
                            vs.iter()
                                .filter_map(|v| {
                                    v.as_str().map(str::to_string).or_else(|| {
                                        v.get("value").and_then(Value::as_str).map(str::to_string)
                                    })
                                })
                                .collect()
                        }),
                    unit: f.get("unit").and_then(Value::as_str).map(str::to_string),
                    currency: f
                        .get("currency")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();
    crate::workspace_routes::SchemaTypeRecord {
        id: alias.clone(),
        name: alias,
        display_name: entry
            .get("label")
            .or_else(|| entry.get("display_name"))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        status: "published".to_string(),
        version: revision as i32,
        description: entry
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        entity_class: entry
            .get("entity_class")
            .and_then(Value::as_str)
            .unwrap_or("data")
            .to_string(),
        entity_count,
        fields,
        preview: entry
            .get("preview")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
        trans: entry.get("trans").and_then(Value::as_bool).unwrap_or(false),
    }
}

/// Apply field-level data permissions to a built view descriptor (1.2):
/// unreadable fields are removed entirely; non-updatable ones render
/// `read_only`. Edge fields are additionally gated by relation-type read (1.4).
fn filter_view_fields(
    view: &mut Value,
    perm: &crate::dataperm::DataPerm,
    tkeys: &[&str],
    ckeys: &[&str],
) {
    let apply = |arr: &mut Vec<Value>| {
        arr.retain_mut(|f| {
            let alias = f
                .get("alias")
                .or_else(|| f.get("predicate_id"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if alias.is_empty() {
                return true;
            }
            // Edge fields (relation predicates) are gated by relation access,
            // scalar fields by field access (1.2 vs 1.4).
            let is_edge = f.get("target_type_id").is_some_and(|v| !v.is_null());
            if is_edge {
                if !perm.can_relation_dir(alias, ckeys, crate::dataperm::Op::Read, Some(true)) {
                    return false;
                }
                if !perm.can_relation_dir(alias, ckeys, crate::dataperm::Op::Update, Some(true)) {
                    if let Some(o) = f.as_object_mut() {
                        o.insert("read_only".to_string(), Value::Bool(true));
                    }
                }
                return true;
            }
            if !perm.can_field(tkeys, ckeys, alias, crate::dataperm::Op::Read) {
                return false;
            }
            if !perm.can_field(tkeys, ckeys, alias, crate::dataperm::Op::Update) {
                if let Some(o) = f.as_object_mut() {
                    o.insert("read_only".to_string(), Value::Bool(true));
                }
            }
            true
        });
    };
    if let Some(sections) = view.get_mut("sections").and_then(Value::as_array_mut) {
        for s in sections {
            if let Some(fields) = s.get_mut("fields").and_then(Value::as_array_mut) {
                apply(fields);
            }
        }
    }
    for key in ["columns", "filters"] {
        if let Some(arr) = view.get_mut(key).and_then(Value::as_array_mut) {
            apply(arr);
        }
    }
}

/// Type alias/type_id lookup entry shared by the catalog readers: an
/// emitted type carries its catalog alias so the UI can disambiguate the
/// same stable_alias published in different catalogs (docs/28 §3).
fn type_record_value(
    entry: &Value,
    revision: i64,
    entity_count: usize,
    catalog_alias: Option<&str>,
) -> Value {
    let mut v = serde_json::to_value(type_entry_to_record(entry, revision, entity_count)).unwrap();
    if let Some(a) = catalog_alias {
        v["catalog"] = Value::String(a.to_string());
    }
    v
}

#[get("/schemas/types?<catalog>")]
pub async fn list_schema_types(
    cap: CapabilityGuard,
    state: &State<AppState>,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id).await?;
            let cat =
                crate::entity_pg::optional_catalog(&mut tx, ctx.tenant_id, catalog.as_deref())
                    .await
                    .map_err(|_| {
                        sf_err(
                            Status::UnprocessableEntity,
                            "catalog.unknown",
                            "Каталог не найден",
                        )
                    })?;
            // An explicit catalog the principal cannot read is a loud
            // denial — same rule as the entity list (docs/28 §4).
            if let Some(c) = &cat {
                let ckeys = [c.public_id.to_string(), c.alias.clone()];
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(
                        Status::Forbidden,
                        "catalog.denied",
                        "Нет доступа к каталогу",
                    ));
                }
            }
            let records =
                published_records(&mut tx, ctx.tenant_id, cat.map(|c| c.catalog_id)).await?;
            let counts: Vec<(uuid::Uuid, i64)> = sqlx::query_as(
                "SELECT type_id, count(*) FROM core.entity
                 WHERE tenant_id=$1 AND retired_at IS NULL GROUP BY type_id",
            )
            .bind(ctx.tenant_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            let mut out = Vec::new();
            for (cid, record) in &records {
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, *cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                // The catalog gate runs before any type check (docs/28 §4).
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    continue;
                }
                let rev = record.revision_id.parse().unwrap_or(0);
                for t in &record.types {
                    let alias = alias_of(t).unwrap_or_default();
                    let tid: Option<(uuid::Uuid,)> = sqlx::query_as(
                        "SELECT type_id FROM meta.type_definition
                         WHERE tenant_id=$1 AND stable_alias=$2 AND catalog_id=$3",
                    )
                    .bind(ctx.tenant_id)
                    .bind(alias)
                    .bind(cid)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| DbError::Other {
                        message: e.to_string(),
                    })?;
                    let count = tid
                        .and_then(|(u,)| counts.iter().find(|(c, _)| c == &u).map(|(_, n)| *n))
                        .unwrap_or(0) as usize;
                    // Data read access on the object type gates its visibility (1.1).
                    let tkeys = [
                        t.get("type_id").and_then(Value::as_str).unwrap_or_default(),
                        alias,
                    ];
                    if !perm.can_type(&tkeys, &ckey_refs, crate::dataperm::Op::Read) {
                        continue;
                    }
                    out.push(type_record_value(
                        t,
                        rev,
                        count,
                        ckeys.get(1).map(String::as_str),
                    ));
                }
            }
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

#[get("/schemas/types/<type_id>?<catalog>")]
pub async fn get_schema_type(
    cap: CapabilityGuard,
    state: &State<AppState>,
    type_id: &str,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let type_id = type_id.to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id).await?;
            let cat =
                crate::entity_pg::optional_catalog(&mut tx, ctx.tenant_id, catalog.as_deref())
                    .await
                    .map_err(|_| {
                        sf_err(
                            Status::UnprocessableEntity,
                            "catalog.unknown",
                            "Каталог не найден",
                        )
                    })?;
            // The same alias may exist in several catalogs; without
            // `?catalog` the first accessible catalog containing it wins.
            let records =
                published_records(&mut tx, ctx.tenant_id, cat.map(|c| c.catalog_id)).await?;
            for (cid, record) in &records {
                let Some(entry) = record.types.iter().find(|t| {
                    alias_of(t) == Some(type_id.as_str())
                        || t.get("type_id").and_then(Value::as_str) == Some(type_id.as_str())
                }) else {
                    continue;
                };
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, *cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                // An unreadable catalog/type behaves as absent (no leak).
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(Status::NotFound, "not_found", "Тип не найден"));
                }
                let alias = alias_of(entry).unwrap_or_default();
                let tkeys = [
                    entry
                        .get("type_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    alias,
                ];
                if !perm.can_type(&tkeys, &ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(Status::NotFound, "not_found", "Тип не найден"));
                }
                let tid: Option<(uuid::Uuid,)> = sqlx::query_as(
                    "SELECT type_id FROM meta.type_definition
                     WHERE tenant_id=$1 AND stable_alias=$2 AND catalog_id=$3",
                )
                .bind(ctx.tenant_id)
                .bind(alias)
                .bind(cid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| DbError::Other {
                    message: e.to_string(),
                })?;
                let entity_count = if let Some((u,)) = tid {
                    sqlx::query_as::<_, (i64,)>(
                        "SELECT count(*) FROM core.entity
                         WHERE tenant_id=$1 AND type_id=$2 AND retired_at IS NULL",
                    )
                    .bind(ctx.tenant_id)
                    .bind(u)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(|e| DbError::Other {
                        message: e.to_string(),
                    })?
                    .0 as usize
                } else {
                    0
                };
                let rev = record.revision_id.parse().unwrap_or(0);
                return Ok(type_record_value(
                    entry,
                    rev,
                    entity_count,
                    ckeys.get(1).map(String::as_str),
                ));
            }
            Err(sf_err(Status::NotFound, "not_found", "Тип не найден"))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

#[get("/schemas/types/<type_id>/view?<view_type>&<catalog>")]
pub async fn get_type_view(
    cap: CapabilityGuard,
    state: &State<AppState>,
    type_id: &str,
    view_type: Option<String>,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let type_id = type_id.to_string();
    let vtype = view_type.unwrap_or_else(|| "form".to_string());
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id).await?;
            let cat =
                crate::entity_pg::optional_catalog(&mut tx, ctx.tenant_id, catalog.as_deref())
                    .await
                    .map_err(|_| {
                        sf_err(
                            Status::UnprocessableEntity,
                            "catalog.unknown",
                            "Каталог не найден",
                        )
                    })?;
            let records =
                published_records(&mut tx, ctx.tenant_id, cat.map(|c| c.catalog_id)).await?;
            for (cid, record) in &records {
                let Some(entry) = record.types.iter().find(|t| {
                    alias_of(t) == Some(type_id.as_str())
                        || t.get("type_id").and_then(Value::as_str) == Some(type_id.as_str())
                }) else {
                    continue;
                };
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, *cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(Status::NotFound, "not_found", "Тип не найден"));
                }
                let alias = alias_of(entry).unwrap_or_default();
                let tkeys = [
                    entry
                        .get("type_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    alias.to_string(),
                ];
                let tkey_refs: [&str; 2] = [&tkeys[0], &tkeys[1]];
                if !perm.can_type(&tkey_refs, &ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(Status::NotFound, "not_found", "Тип не найден"));
                }
                let rev = record.revision_id.parse().unwrap_or(0);
                let schema_type = type_entry_to_record(entry, rev, 0);
                let mut view = crate::workspace_routes::build_type_view(&schema_type, &vtype);
                filter_view_fields(&mut view, &perm, &tkey_refs, &ckey_refs);
                return Ok(view);
            }
            Err(sf_err(Status::NotFound, "not_found", "Тип не найден"))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

#[get("/schemas/predicates?<catalog>")]
pub async fn list_predicates(
    cap: CapabilityGuard,
    state: &State<AppState>,
    catalog: Option<String>,
) -> Result<Json<ApiEnvelope<Value>>, ApiFailure> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = schema_tx(&pool, &ctx).await?;
            let perm =
                crate::dataperm::DataPerm::load(&mut tx, ctx.tenant_id, ctx.principal_id)
                    .await?;
            let cat = crate::entity_pg::optional_catalog(
                &mut tx,
                ctx.tenant_id,
                catalog.as_deref(),
            )
            .await
            .map_err(|_| {
                sf_err(
                    Status::UnprocessableEntity,
                    "catalog.unknown",
                    "Каталог не найден",
                )
            })?;
            // Explicit forbidden catalog — loud denial, same as lists.
            if let Some(c) = &cat {
                let ckeys = [c.public_id.to_string(), c.alias.clone()];
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    return Err(sf_err(
                        Status::Forbidden,
                        "catalog.denied",
                        "Нет доступа к каталогу",
                    ));
                }
            }
            let records =
                published_records(&mut tx, ctx.tenant_id, cat.map(|c| c.catalog_id)).await?;
            let mut out: Vec<Value> = Vec::new();
            for (cid, record) in &records {
                let ckeys = catalog_key_list(&mut tx, ctx.tenant_id, *cid).await?;
                let ckey_refs: Vec<&str> = ckeys.iter().map(String::as_str).collect();
                if !perm.can_catalog(&ckey_refs, crate::dataperm::Op::Read) {
                    continue;
                }
                let catalog_alias = ckeys.get(1).cloned().unwrap_or_default();
                out.extend(
                    record
                        .predicates
                        .iter()
                        .filter(|p| {
                            // 1.4: relation types the principal cannot read are not
                            // disclosed — they must not appear in the catalog at all.
                            let edge =
                                p.get("is_edge").and_then(Value::as_bool).unwrap_or(false);
                            !edge
                                || perm.can_relation(
                                    alias_of(p).unwrap_or_default(),
                                    &ckey_refs,
                                    crate::dataperm::Op::Read,
                                )
                        })
                        .map(|p| {
                            let catalog_alias = catalog_alias.clone();
                            json!({
                                "catalog": catalog_alias,
                        "predicate_id": p.get("predicate_id").or_else(|| p.get("stable_alias")).or_else(|| p.get("alias")).and_then(Value::as_str).unwrap_or_default(),
                        "alias": alias_of(p).unwrap_or_default(),
                        "display_name": p.get("label").or_else(||p.get("display_name")).and_then(Value::as_str).unwrap_or_default(),
                        "label": p.get("label").and_then(Value::as_str).unwrap_or_default(),
                        "is_edge": p.get("is_edge").and_then(Value::as_bool).unwrap_or(false),
                        "direction": p.get("direction").and_then(Value::as_str).unwrap_or("directed"),
                        "domain": p.get("domain").and_then(Value::as_str).unwrap_or_default(),
                        "range": p.get("range").and_then(Value::as_str).unwrap_or_default(),
                        "domain_type": p.get("domain").and_then(Value::as_str).unwrap_or_default(),
                        "range_type": p.get("range").and_then(Value::as_str).unwrap_or_default(),
                        "scalar_kind": p.get("scalar_kind").and_then(Value::as_str).unwrap_or_default(),
                        "min_cardinality": p.get("min_cardinality").and_then(Value::as_i64).unwrap_or(0),
                        "max_cardinality": p.get("max_cardinality").and_then(Value::as_i64).unwrap_or(1),
                        "participates_in_identity": p.get("participates_in_identity").or_else(||p.get("identity")).and_then(Value::as_bool).unwrap_or(false),
                        "confidentiality_class": p.get("confidentiality").or_else(||p.get("confidentiality_class")).and_then(Value::as_str).unwrap_or("internal"),
                        "inverse_label": p.get("inverse_label").and_then(Value::as_str).unwrap_or_default(),
                        "group": p.get("group").and_then(Value::as_str).unwrap_or_default(),
                            })
                        }),
                );
            }
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| failure(&e))?;
    Ok(Json(envelope(v)))
}

pub fn routes() -> Vec<rocket::Route> {
    routes![
        list_drafts,
        get_draft,
        create_draft,
        update_draft,
        delete_draft,
        validate_draft,
        impact_draft,
        publish_draft,
        list_revisions,
        get_revision,
        retire_revision,
        migration_status,
        list_schema_types,
        get_schema_type,
        get_type_view,
        list_predicates,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn draft_with_change() -> SchemaRevisionRecord {
        let mut record = seed_revision();
        record.revision_id = "2".to_string();
        record.status = "draft".to_string();
        record.semantic_version = "1.1.0".to_string();
        record.changes = vec![SchemaChange {
            kind: SchemaChangeKind::Add,
            resource_kind: "predicate".to_string(),
            resource_id: "okved".to_string(),
            summary: "Добавить ОКВЭД".to_string(),
            breaking: false,
        }];
        record.refresh_derived();
        record
    }

    #[test]
    fn valid_alias_rejects_uppercase_and_spaces() {
        assert!(valid_alias("legal_entity"));
        assert!(!valid_alias("LegalEntity"));
        assert!(!valid_alias("legal entity"));
    }

    #[test]
    fn validation_detects_duplicate_aliases() {
        let mut record = draft_with_change();
        record.types.push(record.types[0].clone());
        let report = validate_record(&record);
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "alias.duplicate"));
    }

    #[test]
    fn validation_detects_dangling_field_predicate() {
        let mut record = draft_with_change();
        record.types[0]["fields"] = json!([{"alias": "no_such_predicate", "kind": "text"}]);
        let report = validate_record(&record);
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "field.predicate.unknown" && f.severity == "error"));
    }

    #[test]
    fn validation_detects_dangling_edge_range_and_missing_domain() {
        let mut record = draft_with_change();
        record.predicates.push(json!({
            "stable_alias": "bad_edge",
            "is_edge": true,
            "domain": "ghost_type",
            "range": "another_ghost"
        }));
        record.predicates.push(json!({
            "stable_alias": "halved_edge",
            "is_edge": true
        }));
        let report = validate_record(&record);
        assert!(!report.valid);
        let codes: Vec<&str> = report.findings.iter().map(|f| f.code.as_str()).collect();
        assert!(codes.contains(&"edge.domain.unknown"));
        assert!(codes.contains(&"edge.range.unknown"));
        assert!(codes.contains(&"edge.domain.required"));
        assert!(codes.contains(&"edge.range.required"));
    }

    #[test]
    fn validation_detects_dangling_field_target_type() {
        let mut record = draft_with_change();
        let pred_alias = record.predicates[0]
            .get("stable_alias")
            .and_then(Value::as_str)
            .unwrap()
            .to_string();
        record.types[0]["fields"] = json!([
            {"alias": pred_alias, "kind": "entity_reference", "target_type_id": "ghost"}
        ]);
        let report = validate_record(&record);
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "field.target_type.unknown" && f.severity == "error"));
    }

    #[test]
    fn validation_warns_on_unknown_relation_group() {
        let mut record = draft_with_change();
        record.predicates.push(json!({
            "stable_alias": "grouped_edge",
            "is_edge": true,
            "domain": record.types[0]
                .get("stable_alias")
                .and_then(Value::as_str)
                .unwrap_or("person"),
            "range": record.types[0]
                .get("stable_alias")
                .and_then(Value::as_str)
                .unwrap_or("person"),
            "group": "no_such_group"
        }));
        let report = validate_record(&record);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "edge.group.unknown" && f.severity == "warning"));
    }

    #[test]
    fn additive_change_is_not_breaking() {
        let record = draft_with_change();
        let parent = seed_revision();
        let report = impact_record(&record, Some(&parent));
        assert_eq!(report.compatibility, "additive");
        assert!(!report.breaking);
        assert_eq!(report.details.new_fields, 1);
    }

    #[test]
    fn retire_change_requires_migration() {
        let mut record = draft_with_change();
        record.changes[0].kind = SchemaChangeKind::Retire;
        let report = impact_record(&record, Some(&seed_revision()));
        assert!(report.breaking);
        assert!(report.details.migration_required);
    }

    #[test]
    fn content_hash_is_stable_and_sensitive_to_content() {
        let first = draft_with_change();
        let mut second = first.clone();
        assert_eq!(content_hash(&first), content_hash(&second));
        second.semantic_version = "1.2.0".to_string();
        second.changes[0].summary = "Different review text".to_string();
        assert_eq!(content_hash(&first), content_hash(&second));
        second.types.push(json!({
            "type_id": "contract",
            "stable_alias": "contract",
            "entity_class": "data"
        }));
        assert_ne!(content_hash(&first), content_hash(&second));
        assert_eq!(seed_revision().content_hash.unwrap().len(), 64);
    }

    #[test]
    fn validation_rejects_incomplete_change_log_entries() {
        let mut record = draft_with_change();
        record.changes[0].resource_id.clear();
        record.changes[0].summary.clear();
        let report = validate_record(&record);
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "change.resource_id.required"));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.code == "change.summary.required"));
    }

    #[test]
    fn impact_uses_definition_diff_instead_of_only_change_log() {
        let parent = seed_revision();
        let mut record = draft_with_change();
        record.changes.clear();
        record.predicates.push(json!({
            "predicate_id": "okved",
            "stable_alias": "okved",
            "is_edge": false,
            "scalar_kind": "text"
        }));
        let report = impact_record(&record, Some(&parent));
        assert_eq!(report.details.new_fields, 1);
        assert_eq!(report.compatibility, "additive");
    }

    #[test]
    fn reason_has_minimum_length() {
        assert!(validate_reason("short").is_err());
        assert!(validate_reason("Approved change request CR-42").is_ok());
    }

    #[test]
    fn validation_rejects_unknown_field_kind() {
        let mut record = seed_revision();
        record.types[0] = json!({
            "type_id": "person",
            "stable_alias": "person",
            "entity_class": "data",
            "fields": [
                { "alias": "full_name", "kind": "text", "required": true },
                { "alias": "tags", "kind": "tagz" },
                { "alias": "status", "kind": "enum" },
                { "alias": "manager", "kind": "entity_reference" }
            ]
        });
        let report = validate_record(&record);
        assert!(!report.valid);
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "field.kind.unknown"
                && f.severity == "error"
                && f.path.as_deref() == Some("types[0].fields[1].kind")));
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "field.values.recommended" && f.severity == "warning"));
        assert!(report
            .findings
            .iter()
            .any(|f| f.code == "field.target_type.recommended" && f.severity == "warning"));
    }

    #[test]
    fn validation_accepts_all_documented_field_kinds() {
        let mut record = seed_revision();
        let fields: Vec<Value> = crate::field_kinds::KNOWN_FIELD_KINDS
            .iter()
            .map(|k| json!({ "alias": format!("f_{k}"), "kind": k }))
            .collect();
        record.types[0] = json!({
            "type_id": "person",
            "stable_alias": "person",
            "entity_class": "data",
            "fields": fields
        });
        let report = validate_record(&record);
        assert!(!report
            .findings
            .iter()
            .any(|f| f.code == "field.kind.unknown"));
    }
}
