//! PostgreSQL-backed entity workspace handlers (docs/04, docs/10).
//!
//! Every handler runs inside a transaction with the tenant RLS context
//! installed transaction-locally, so all `core.*`/`meta.*` queries are tenant
//! isolated at the database level. Results cross the async-std executor
//! boundary as `serde_json::Value`.

use crate::authn::AuthContext;
use crate::workspace_routes::{
    meta, ApiEnvelope, CapabilityGuard, EntityRecord, GraphDataRecord, GraphEdgeRecord,
    GraphNodeRecord, HistoryChangeRecord, HistoryEntryRecord, QueryResponseRecord, RelationRecord,
};
use crate::AppState;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use enginer_domain::{CanonicalScalar, PredicateId, StatementObject, TenantId};
use enginer_persistence::RlsContext;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use serde_json::{json, Map, Value};
use sqlx::postgres::PgPool;
use sqlx::Postgres;
use sqlx::Transaction;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::sync::Arc;

pub(crate) fn db_err(e: impl Display) -> DbError {
    DbError::Other {
        message: e.to_string(),
    }
}

/// Encode an HTTP status into the DbError so handlers can map it back.
pub(crate) fn http_err(code: u16, msg: &str) -> DbError {
    DbError::Other {
        message: format!("HTTP{code}:{msg}"),
    }
}

pub(crate) fn to_status(e: &DbError) -> Status {
    if let DbError::Other { message } = e {
        if let Some(rest) = message.strip_prefix("HTTP") {
            if let Some(code) = rest.split(':').next().and_then(|c| c.parse::<u16>().ok()) {
                return Status::from_code(code).unwrap_or(Status::InternalServerError);
            }
        }
    }
    Status::InternalServerError
}

pub(crate) async fn rls_tx<'e>(
    pool: &'e PgPool,
    ctx: &AuthContext,
    purpose: &str,
) -> Result<Transaction<'e, Postgres>, DbError> {
    let mut tx = pool.begin().await.map_err(db_err)?;
    RlsContext {
        tenant_id: TenantId(ctx.tenant_id),
        principal_id: enginer_domain::PrincipalId(ctx.principal_id),
        purpose: purpose.to_string(),
        policy_revision: None,
    }
    .install_tx(&mut tx)
    .await
    .map_err(|e| db_err(e.to_string()))?;
    Ok(tx)
}

fn ok_json(v: Value) -> Result<Value, DbError> {
    Ok(v)
}

// ---------------------------------------------------------------------------
// Read model
// ---------------------------------------------------------------------------

/// Convert a stored canonical object into the wire field value.
pub(crate) fn object_to_wire(obj: &StatementObject) -> Value {
    match obj {
        StatementObject::Entity(id) => json!({ "entity_id": id.raw() }),
        StatementObject::Scalar(s) => match s {
            CanonicalScalar::Text { value_nfc, .. } => Value::String(value_nfc.clone()),
            CanonicalScalar::Boolean { value } => Value::Bool(*value),
            CanonicalScalar::Integer { value } => Value::String(value.to_string()),
            CanonicalScalar::Decimal { value } => Value::String(value.clone()),
            CanonicalScalar::Date { value } => Value::String(value.to_string()),
            CanonicalScalar::Instant { value, .. } => Value::String(value.to_rfc3339()),
            CanonicalScalar::Uuid { value } => Value::String(value.to_string()),
            CanonicalScalar::Uri { value } => Value::String(value.clone()),
            CanonicalScalar::Email { local, domain } => Value::String(format!("{local}@{domain}")),
            CanonicalScalar::Phone { e164, extension } => extension.as_ref().map_or_else(
                || Value::String(e164.clone()),
                |ext| Value::String(format!("{e164}x{ext}")),
            ),
            CanonicalScalar::Money { amount, currency } => {
                json!({ "amount": amount, "currency": currency })
            }
            CanonicalScalar::Measurement { amount, unit } => {
                json!({ "amount": amount, "unit": unit })
            }
            CanonicalScalar::Json { value } => {
                serde_json::from_str(value).unwrap_or_else(|_| Value::String(value.clone()))
            }
            CanonicalScalar::Blob { content_hash, .. } => {
                json!({ "blob": hex::encode(content_hash.0) })
            }
        },
    }
}

pub(crate) fn wire_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Object(o) => o
            .get("value")
            .or_else(|| o.get("value_nfc"))
            .and_then(wire_text),
        _ => None,
    }
}

pub(crate) fn display_name_of(
    type_alias: &str,
    fields: &Map<String, Value>,
    entity_id: i64,
) -> String {
    ["display_name", "short_name", "full_name", "name", "title"]
        .iter()
        .find_map(|k| fields.get(*k).and_then(wire_text))
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| format!("{type_alias} #{entity_id}"))
}

/// Load active scalar fields for a batch of entities (one query).
pub(crate) async fn load_fields(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    entity_ids: &[i64],
) -> Result<HashMap<i64, Map<String, Value>>, DbError> {
    let mut out: HashMap<i64, Map<String, Value>> = HashMap::new();
    if entity_ids.is_empty() {
        return Ok(out);
    }
    let rows: Vec<(i64, String, Value)> = sqlx::query_as(
        "SELECT s.subject, pd.stable_alias, s.object
         FROM core.statement s
         JOIN meta.predicate_definition pd
           ON pd.tenant_id = s.tenant_id AND pd.predicate_id = s.predicate
         WHERE s.tenant_id = $1 AND s.subject = ANY($2)
           AND s.status = 'active' AND s.object_kind = 'scalar'
           AND (s.valid_to IS NULL OR s.valid_to > now())",
    )
    .bind(tenant)
    .bind(entity_ids)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    for (subject, alias, obj_json) in rows {
        let Ok(obj) = serde_json::from_value::<StatementObject>(obj_json) else {
            continue;
        };
        out.entry(subject)
            .or_default()
            .insert(alias, object_to_wire(&obj));
    }
    Ok(out)
}

pub(crate) struct EntityRow {
    pub entity_id: i64,
    pub public_id: uuid::Uuid,
    #[allow(dead_code)]
    pub type_id: uuid::Uuid,
    pub type_alias: String,
    pub entity_class: i16,
    pub head_revision: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub retired_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub(crate) const ENTITY_SELECT: &str = "SELECT e.entity_id, e.public_id, e.type_id,
        COALESCE(td.stable_alias, e.type_id::text) AS type_alias,
        e.entity_class, e.head_revision, e.created_at, e.retired_at
     FROM core.entity e
     LEFT JOIN meta.type_definition td
       ON td.tenant_id = e.tenant_id AND td.type_id = e.type_id
     WHERE e.tenant_id = $1";

pub(crate) fn row_to_entity_row(
    r: (
        i64,
        uuid::Uuid,
        uuid::Uuid,
        String,
        i16,
        i64,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    ),
) -> EntityRow {
    EntityRow {
        entity_id: r.0,
        public_id: r.1,
        type_id: r.2,
        type_alias: r.3,
        entity_class: r.4,
        head_revision: r.5,
        created_at: r.6,
        retired_at: r.7,
    }
}

pub(crate) fn entity_class_str(c: i16) -> &'static str {
    match c {
        0 => "data",
        1 => "schema",
        2 => "policy",
        3 => "workflow",
        _ => "system",
    }
}

/// Load the data-permission set for the request's principal inside an open
/// tenant transaction.
pub(crate) async fn perm_for(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
) -> Result<crate::dataperm::DataPerm, DbError> {
    crate::dataperm::DataPerm::load(tx, ctx.tenant_id, ctx.principal_id).await
}

/// Type keys used by rule lookups: both the type uuid and its stable alias
/// are accepted as `resource.type` values.
pub(crate) fn type_keys(row: &EntityRow) -> [String; 2] {
    [row.type_id.to_string(), row.type_alias.clone()]
}

/// True when the entity row is readable under `perm` (type read + instance
/// read; denied entities are indistinguishable from absent ones).
pub(crate) fn entity_visible(perm: &crate::dataperm::DataPerm, row: &EntityRow) -> bool {
    let keys = type_keys(row);
    let key_refs: [&str; 2] = [&keys[0], &keys[1]];
    perm.can_type(&key_refs, crate::dataperm::Op::Read)
        && perm.can_entity(&row.public_id.to_string(), crate::dataperm::Op::Read)
}

/// Resolve an entity by public_id under RLS and require DataPerm access.
/// `Op::Read` for reads of entity-linked resources (documents, checks,
/// factors, tasks), `Op::Update` for mutations. A denied entity is
/// indistinguishable from a missing one (404) — same contract as the
/// entity routes.
pub(crate) async fn checked_entity(
    tx: &mut Transaction<'_, Postgres>,
    ctx: &AuthContext,
    public_id: uuid::Uuid,
    op: crate::dataperm::Op,
) -> Result<EntityRow, DbError> {
    let perm = perm_for(tx, ctx).await?;
    let row: Option<(
        i64,
        uuid::Uuid,
        uuid::Uuid,
        String,
        i16,
        i64,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
        .bind(ctx.tenant_id)
        .bind(public_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;
    let erow = row_to_entity_row(row.ok_or_else(|| http_err(404, "entity not found"))?);
    let allowed = if op == crate::dataperm::Op::Read {
        entity_visible(&perm, &erow)
    } else {
        let keys = type_keys(&erow);
        let key_refs: [&str; 2] = [&keys[0], &keys[1]];
        perm.can_type(&key_refs, op) && perm.can_entity(&public_id.to_string(), op)
    };
    if !allowed {
        return Err(http_err(404, "entity not found"));
    }
    Ok(erow)
}

pub(crate) async fn record_for(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    rows: &[EntityRow],
    perm: &crate::dataperm::DataPerm,
) -> Result<Vec<EntityRecord>, DbError> {
    let visible: Vec<&EntityRow> = rows.iter().filter(|r| entity_visible(perm, r)).collect();
    let ids: Vec<i64> = visible.iter().map(|r| r.entity_id).collect();
    let mut fields_map = load_fields(tx, tenant, &ids).await?;
    Ok(visible
        .iter()
        .map(|r| {
            let r = *r;
            let mut fields = fields_map.remove(&r.entity_id).unwrap_or_default();
            let keys = type_keys(r);
            let key_refs: [&str; 2] = [&keys[0], &keys[1]];
            perm.filter_fields(&key_refs, &mut fields);
            perm.mask_fields(&key_refs, &mut fields);
            EntityRecord {
                public_id: r.public_id.to_string(),
                type_id: r.type_alias.clone(),
                entity_class: entity_class_str(r.entity_class).to_string(),
                revision: r.head_revision,
                display_name: display_name_of(&r.type_alias, &fields, r.entity_id),
                status: if r.retired_at.is_some() {
                    "retired".to_string()
                } else {
                    "active".to_string()
                },
                created_at: r.created_at.to_rfc3339(),
                updated_at: r
                    .retired_at
                    .unwrap_or(r.created_at)
                    .max(r.created_at)
                    .to_rfc3339(),
                fields: Some(Value::Object(fields)),
            }
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Write model helpers
// ---------------------------------------------------------------------------

pub(crate) async fn next_val(
    tx: &mut Transaction<'_, Postgres>,
    seq: &str,
) -> Result<i64, DbError> {
    let (v,): (i64,) = sqlx::query_as(&format!("SELECT nextval('{seq}')"))
        .fetch_one(&mut **tx)
        .await
        .map_err(db_err)?;
    Ok(v)
}

/// Convert a wire `{kind, ...}` value into a canonical statement object.
/// `resolve_entity` resolves a target public_id to its internal id for
/// `entity` values.
pub(crate) async fn input_to_object(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    is_edge: bool,
    input: &Value,
) -> Result<StatementObject, DbError> {
    let kind = input.get("kind").and_then(Value::as_str).unwrap_or("text");
    if is_edge || kind == "entity" {
        let public = input
            .get("entity_public_id")
            .or_else(|| input.get("value"))
            .and_then(Value::as_str)
            .ok_or_else(|| http_err(422, "edge value requires entity_public_id"))?;
        let target: Option<(i64,)> = sqlx::query_as(
            "SELECT entity_id FROM core.entity WHERE tenant_id = $1 AND public_id = $2",
        )
        .bind(tenant)
        .bind(
            public
                .parse::<uuid::Uuid>()
                .map_err(|_| http_err(422, "bad target id"))?,
        )
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;
        let (eid,) = target.ok_or_else(|| http_err(422, "target entity not found"))?;
        return Ok(StatementObject::Entity(enginer_domain::EntityId(eid)));
    }
    let scalar = match kind {
        "boolean" => CanonicalScalar::Boolean {
            value: input.get("value").and_then(Value::as_bool).unwrap_or(false),
        },
        "integer" => CanonicalScalar::Integer {
            value: input
                .get("value")
                .and_then(|v| {
                    v.as_i64()
                        .map(i128::from)
                        .or_else(|| v.as_str().and_then(|s| s.parse::<i128>().ok()))
                })
                .ok_or_else(|| http_err(422, "bad integer value"))?,
        },
        "decimal" => CanonicalScalar::Decimal {
            value: input
                .get("value")
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| v.to_string())
                })
                .ok_or_else(|| http_err(422, "bad decimal value"))?,
        },
        "date" => CanonicalScalar::Date {
            value: input
                .get("value")
                .and_then(Value::as_str)
                .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
                .ok_or_else(|| http_err(422, "bad date value"))?,
        },
        "instant" => CanonicalScalar::Instant {
            value: input
                .get("value")
                .and_then(Value::as_str)
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|d| d.with_timezone(&chrono::Utc))
                .ok_or_else(|| http_err(422, "bad instant value"))?,
            source_zone: None,
        },
        "uuid" => CanonicalScalar::Uuid {
            value: input
                .get("value")
                .and_then(Value::as_str)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| http_err(422, "bad uuid value"))?,
        },
        "uri" => CanonicalScalar::Uri {
            value: input
                .get("value")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        },
        "email" => CanonicalScalar::Email {
            local: input
                .get("local")
                .and_then(Value::as_str)
                .or_else(|| {
                    input
                        .get("value")
                        .and_then(Value::as_str)
                        .and_then(|s| s.split('@').next())
                })
                .unwrap_or_default()
                .to_string(),
            domain: input
                .get("domain")
                .and_then(Value::as_str)
                .or_else(|| {
                    input
                        .get("value")
                        .and_then(Value::as_str)
                        .and_then(|s| s.split('@').nth(1))
                })
                .unwrap_or_default()
                .to_string(),
        },
        "phone" => CanonicalScalar::Phone {
            e164: input
                .get("e164")
                .and_then(Value::as_str)
                .or_else(|| input.get("value").and_then(Value::as_str))
                .unwrap_or_default()
                .to_string(),
            extension: input
                .get("extension")
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "json" => CanonicalScalar::Json {
            value: serde_json::to_string(&input.get("value").cloned().unwrap_or(Value::Null))
                .unwrap_or_else(|_| "null".to_string()),
        },
        _ => CanonicalScalar::Text {
            value_nfc: input
                .get("value")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    input.get("value").and_then(|v| {
                        (!v.is_null() && !v.is_string() && !v.is_object() && !v.is_array())
                            .then(|| v.to_string())
                    })
                })
                .unwrap_or_default(),
            language_tag: None,
        },
    };
    Ok(StatementObject::Scalar(scalar))
}

/// Extract a normalized identity component from a canonical scalar (mirrors
/// `scalar_key_component` in the CreateEntity use case).
fn scalar_key_component(s: &CanonicalScalar) -> Option<String> {
    let component = match s {
        CanonicalScalar::Text { value_nfc, .. } => value_nfc.clone(),
        CanonicalScalar::Email { local, domain } => format!("{local}@{domain}"),
        CanonicalScalar::Phone { e164, .. } => e164.clone(),
        CanonicalScalar::Integer { value } => value.to_string(),
        CanonicalScalar::Uuid { value } => value.to_string(),
        CanonicalScalar::Uri { value } => value.clone(),
        _ => return None,
    };
    // A blank component is equivalent to a missing one: hashing "" would make
    // every entity with an empty optional identity field collide.
    (!component.trim().is_empty()).then_some(component)
}

/// Compute (policy_id, key) pairs for an entity whose scalar objects are
/// indexed by predicate id. A policy contributes a key only when every
/// component resolves to a non-empty canonical scalar.
fn compute_policy_keys(
    policies: &[(uuid::Uuid, Vec<uuid::Uuid>)],
    by_pred: &HashMap<uuid::Uuid, StatementObject>,
) -> Vec<(uuid::Uuid, String)> {
    let mut keys = Vec::new();
    for (policy_id, components) in policies {
        if components.is_empty() {
            continue;
        }
        let mut parts = Vec::with_capacity(components.len());
        let mut complete = true;
        for comp in components {
            match by_pred.get(comp) {
                Some(StatementObject::Scalar(s)) => match scalar_key_component(s) {
                    Some(part) => parts.push(part),
                    None => {
                        complete = false;
                        break;
                    }
                },
                _ => {
                    complete = false;
                    break;
                }
            }
        }
        if complete {
            keys.push((*policy_id, enginer_domain::compute_identity_key(&parts)));
        }
    }
    keys
}

/// After a statement update, re-derive the entity's identity keys: retire
/// keys that no longer hold, insert newly complete keys, and fail with 409
/// when a new key is already owned by another active entity.
async fn recompute_identity_keys(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    entity_id: i64,
    type_id: uuid::Uuid,
) -> Result<(), DbError> {
    let policies: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = sqlx::query_as(
        "SELECT policy_id, key_component_predicate_ids FROM meta.identity_policy
         WHERE tenant_id=$1 AND type_id=$2",
    )
    .bind(tenant)
    .bind(type_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;

    let rows: Vec<(uuid::Uuid, Value)> = sqlx::query_as(
        "SELECT predicate, object FROM core.statement
         WHERE tenant_id=$1 AND subject=$2 AND status='active' AND object_kind='scalar'",
    )
    .bind(tenant)
    .bind(entity_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    let mut by_pred: HashMap<uuid::Uuid, StatementObject> = HashMap::new();
    for (predicate, obj_json) in rows {
        if let Ok(obj) = serde_json::from_value::<StatementObject>(obj_json) {
            by_pred.insert(predicate, obj);
        }
    }
    let new_keys = compute_policy_keys(&policies, &by_pred);

    // Collision: another active entity already owns this key.
    for (policy_id, key) in &new_keys {
        let other: Option<(i64,)> = sqlx::query_as(
            "SELECT k.entity_id FROM core.identity_key k
             JOIN core.entity e
               ON e.tenant_id=k.tenant_id AND e.entity_id=k.entity_id
             WHERE k.tenant_id=$1 AND k.type_id=$2 AND k.policy_id=$3 AND k.key=$4
               AND k.status=0 AND k.entity_id<>$5 AND e.retired_at IS NULL",
        )
        .bind(tenant)
        .bind(type_id)
        .bind(policy_id)
        .bind(key)
        .bind(entity_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?;
        if other.is_some() {
            return Err(http_err(409, "identity collision"));
        }
    }

    // Retire keys that are no longer derivable.
    let existing: Vec<(uuid::Uuid, String)> = sqlx::query_as(
        "SELECT policy_id, key FROM core.identity_key
         WHERE tenant_id=$1 AND entity_id=$2 AND status=0",
    )
    .bind(tenant)
    .bind(entity_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    for (policy_id, key) in &existing {
        if !new_keys.iter().any(|(p, k)| p == policy_id && k == key) {
            sqlx::query(
                "UPDATE core.identity_key SET status=1
                 WHERE tenant_id=$1 AND entity_id=$2 AND policy_id=$3 AND key=$4",
            )
            .bind(tenant)
            .bind(entity_id)
            .bind(policy_id)
            .bind(key)
            .execute(&mut **tx)
            .await
            .map_err(db_err)?;
        }
    }

    // Insert or reactivate keys.
    for (policy_id, key) in &new_keys {
        sqlx::query(
            "INSERT INTO core.identity_key (tenant_id, type_id, policy_id, key, entity_id, status)
             VALUES ($1,$2,$3,$4,$5,0)
             ON CONFLICT (tenant_id, type_id, policy_id, key)
             DO UPDATE SET status=0, entity_id=$5",
        )
        .bind(tenant)
        .bind(type_id)
        .bind(policy_id)
        .bind(key)
        .bind(entity_id)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

async fn insert_statement(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    stmt_id: i64,
    subject: i64,
    predicate: uuid::Uuid,
    object: &StatementObject,
    valid_from: chrono::DateTime<chrono::Utc>,
    actor: uuid::Uuid,
    revision: i64,
) -> Result<(), DbError> {
    let obj_json = serde_json::to_value(object).map_err(db_err)?;
    let fingerprint = enginer_domain::fact_fingerprint(
        TenantId(tenant),
        subject,
        PredicateId(predicate),
        object,
        &enginer_domain::ValidPeriod {
            valid_from,
            valid_to: None,
        },
        None,
        None,
    )
    .to_hex();
    sqlx::query(
        "INSERT INTO core.statement
         (tenant_id, statement_id, subject, predicate, object, object_kind,
          valid_from, recorded_at, provenance, confidence, status, revision, fingerprint)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,'1.0','active',$10,$11)",
    )
    .bind(tenant)
    .bind(stmt_id)
    .bind(subject)
    .bind(predicate)
    .bind(obj_json)
    .bind(if object.is_edge() { "entity" } else { "scalar" })
    .bind(valid_from)
    .bind(valid_from)
    .bind(serde_json::json!({ "kind": "user", "principal_id": actor }))
    .bind(revision)
    .bind(fingerprint)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn bump_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    entity_id: i64,
    actor: uuid::Uuid,
    command: &str,
    schema_revision: Option<i64>,
) -> Result<i64, DbError> {
    let (rev,): (i64,) = sqlx::query_as(
        "UPDATE core.entity SET head_revision = head_revision + 1
         WHERE tenant_id = $1 AND entity_id = $2 RETURNING head_revision",
    )
    .bind(tenant)
    .bind(entity_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    sqlx::query(
        "INSERT INTO core.entity_revision
         (tenant_id, entity_id, revision, schema_revision_id, actor, command, previous_revision)
         VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(tenant)
    .bind(entity_id)
    .bind(rev)
    .bind(schema_revision)
    .bind(actor)
    .bind(command)
    .bind(rev - 1)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(rev)
}

async fn outbox_event(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    aggregate_id: &str,
    revision: i64,
    event_type: &str,
    payload: Value,
) -> Result<(), DbError> {
    sqlx::query(
        "INSERT INTO ops.outbox (tenant_id, aggregate_kind, aggregate_id, revision, event_type, payload)
         VALUES ($1,'entity',$2,$3,$4,$5)",
    )
    .bind(tenant)
    .bind(aggregate_id)
    .bind(revision)
    .bind(event_type)
    .bind(payload)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(())
}

async fn published_schema_revision(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
) -> Result<Option<i64>, DbError> {
    let row: Option<(i64,)> = sqlx::query_as(
        "SELECT revision_id FROM meta.schema_revision WHERE tenant_id=$1 AND state='published'",
    )
    .bind(tenant)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(row.map(|r| r.0))
}

async fn resolve_type(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    reference: &str,
) -> Result<(uuid::Uuid, String), DbError> {
    let row: Option<(uuid::Uuid, String)> = if let Ok(id) = reference.parse::<uuid::Uuid>() {
        sqlx::query_as(
            "SELECT type_id, stable_alias FROM meta.type_definition
             WHERE tenant_id=$1 AND type_id=$2 AND retired_revision IS NULL",
        )
        .bind(tenant)
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?
    } else {
        sqlx::query_as(
            "SELECT type_id, stable_alias FROM meta.type_definition
             WHERE tenant_id=$1 AND stable_alias=$2 AND retired_revision IS NULL",
        )
        .bind(tenant)
        .bind(reference)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_err)?
    };
    row.ok_or_else(|| http_err(422, "unknown type"))
}

async fn resolve_predicates(
    tx: &mut Transaction<'_, Postgres>,
    tenant: uuid::Uuid,
    aliases: &[String],
) -> Result<HashMap<String, (uuid::Uuid, bool)>, DbError> {
    let rows: Vec<(String, uuid::Uuid, bool)> = sqlx::query_as(
        "SELECT stable_alias, predicate_id, is_edge FROM meta.predicate_definition
         WHERE tenant_id=$1 AND retired_revision IS NULL
           AND (stable_alias = ANY($2) OR predicate_id::text = ANY($2))",
    )
    .bind(tenant)
    .bind(aliases)
    .fetch_all(&mut **tx)
    .await
    .map_err(db_err)?;
    let mut map = HashMap::new();
    for (alias, pid, edge) in rows {
        map.insert(alias, (pid, edge));
    }
    Ok(map)
}

fn exec_of(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, Status> {
    state.db_executor.clone().ok_or(Status::ServiceUnavailable)
}

fn run_err(e: DbError) -> Status {
    let status = to_status(&e);
    if status.code >= 500 {
        tracing::error!(error = %e, "entity_pg db error");
    }
    status
}

fn json_to<T: serde::de::DeserializeOwned>(v: Value) -> Result<T, Status> {
    serde_json::from_value(v).map_err(|_| Status::InternalServerError)
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `GET /entities` — page through entities by internal id cursor.
#[get("/entities?<type>&<search>&<limit>&<cursor>")]
pub async fn pg_query_entities(
    cap: CapabilityGuard,
    state: &State<AppState>,
    r#type: Option<String>,
    search: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Result<Json<ApiEnvelope<Vec<EntityRecord>>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let limit = limit.unwrap_or(50).clamp(1, 200) as i64;
    let after: i64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
    let search = search.map(|s| s.trim().to_lowercase());
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.list").await?;
            let perm = perm_for(&mut tx, &ctx).await?;
            // When searching, load a bounded batch and filter in memory —
            // display_name and field values live in statement rows.
            let fetch_limit = if search.is_some() { 2000 } else { limit };
            let rows: Vec<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = if let Some(t) = r#type.as_deref() {
                sqlx::query_as(&format!(
                    "{ENTITY_SELECT} AND (td.stable_alias = $2 OR e.type_id::text = $2)
                       AND e.entity_id > $3 AND e.retired_at IS NULL
                     ORDER BY e.entity_id LIMIT $4"
                ))
                .bind(ctx.tenant_id)
                .bind(t)
                .bind(after)
                .bind(fetch_limit)
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?
            } else {
                sqlx::query_as(&format!(
                    "{ENTITY_SELECT} AND e.entity_id > $2 AND e.retired_at IS NULL
                     ORDER BY e.entity_id LIMIT $3"
                ))
                .bind(ctx.tenant_id)
                .bind(after)
                .bind(fetch_limit)
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?
            };
            let rows: Vec<EntityRow> = rows.into_iter().map(row_to_entity_row).collect();
            let mut records = record_for(&mut tx, ctx.tenant_id, &rows, &perm).await?;
            if let Some(needle) = &search {
                records.retain(|e| {
                    e.display_name.to_lowercase().contains(needle)
                        || e.fields
                            .as_ref()
                            .and_then(Value::as_object)
                            .is_some_and(|o| {
                                o.values()
                                    .filter_map(wire_text)
                                    .any(|s| s.to_lowercase().contains(needle))
                            })
                });
                records.sort_by(|a, b| a.display_name.cmp(&b.display_name));
                records.truncate(limit as usize);
            }
            ok_json(serde_json::to_value(records).unwrap_or(Value::Null))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

/// `GET /entities/{public_id}`.
#[get("/entities/<public_id>")]
pub async fn pg_get_entity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<EntityRecord>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.get").await?;
            let perm = perm_for(&mut tx, &ctx).await?;
            let row: Option<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
                .bind(ctx.tenant_id)
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
            let row = row_to_entity_row(row.ok_or_else(|| http_err(404, "entity not found"))?);
            if !entity_visible(&perm, &row) {
                return Err(http_err(404, "entity not found"));
            }
            let rows = vec![row];
            let records = record_for(&mut tx, ctx.tenant_id, &rows, &perm).await?;
            ok_json(serde_json::to_value(&records[0]).unwrap_or(Value::Null))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

/// `POST /entities` — create entity with statements + dedup via identity keys.
#[post("/entities", data = "<body>")]
pub async fn pg_create_entity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let body = body.into_inner();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.create").await?;
            let tenant = ctx.tenant_id;
            let actor = ctx.principal_id;

            let type_ref = body
                .get("type_id")
                .and_then(Value::as_str)
                .ok_or_else(|| http_err(422, "type_id required"))?;
            let (type_id, type_alias) = resolve_type(&mut tx, tenant, type_ref).await?;

            // Data-access check (1.1): create on the object type.
            let perm = perm_for(&mut tx, &ctx).await?;
            let tkeys = [type_id.to_string(), type_alias.clone()];
            if !perm.can_type(
                &[&tkeys[0], &tkeys[1]],
                crate::dataperm::Op::Create,
            ) {
                return Err(http_err(403, "no create access to type"));
            }

            let published = published_schema_revision(&mut tx, tenant).await?;
            let requested_rev = body
                .get("schema_revision")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            if let Some(pub_rev) = published {
                if requested_rev > 0 && requested_rev > pub_rev {
                    return Err(http_err(422, "schema_revision not published"));
                }
            }
            let identity_mode = body
                .get("identity_mode")
                .and_then(Value::as_str)
                .unwrap_or("return_existing");
            let statements = body
                .get("statements")
                .and_then(Value::as_array)
                .cloned()
                .or_else(|| {
                    body.get("fields")
                        .and_then(Value::as_object)
                        .map(|o| {
                            o.iter()
                                .map(|(k, v)| {
                                    json!({ "predicate_id": k, "value": { "kind": "text", "value": v } })
                                })
                                .collect()
                        })
                })
                .unwrap_or_default();

            let aliases: Vec<String> = statements
                .iter()
                .filter_map(|s| s.get("predicate_id").and_then(Value::as_str).map(str::to_string))
                .collect();
            let preds = resolve_predicates(&mut tx, tenant, &aliases).await?;

            // Build canonical objects.
            let mut objects: Vec<(uuid::Uuid, StatementObject)> = Vec::new();
            for stmt in &statements {
                let alias = stmt
                    .get("predicate_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let input = stmt.get("value").cloned().unwrap_or(Value::Null);
                let (pid, is_edge) = preds
                    .get(alias)
                    .copied()
                    .or_else(|| {
                        alias
                            .parse::<uuid::Uuid>()
                            .ok()
                            .map(|u| (u, input.get("kind").and_then(Value::as_str) == Some("entity")))
                    })
                    .ok_or_else(|| http_err(422, "unknown predicate"))?;
                // Edge statements need relation-create on the predicate type;
                // scalars need field-create on the object type (1.2/1.4).
                let allowed = if is_edge {
                    perm.can_relation_dir(alias, crate::dataperm::Op::Create, Some(true))
                } else {
                    perm.can_field(&[&tkeys[0], &tkeys[1]], alias, crate::dataperm::Op::Create)
                };
                if !allowed {
                    return Err(http_err(403, "no write access to field"));
                }
                let obj = input_to_object(&mut tx, tenant, is_edge, &input).await?;
                objects.push((pid, obj));
            }

            // Required-field validation against the published type
            // definition (`fields[].required`). Blank text values count as
            // missing — a whitespace-only required field is not satisfied.
            if let Some(rev) = published {
                let content: Option<Value> = sqlx::query_scalar(
                    "SELECT content FROM meta.schema_revision
                     WHERE tenant_id=$1 AND revision_id=$2",
                )
                .bind(tenant)
                .bind(rev)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
                let type_id_str = type_id.to_string();
                let type_def = content
                    .as_ref()
                    .and_then(|c| c.get("types"))
                    .and_then(Value::as_array)
                    .and_then(|types| {
                        types.iter().find(|t| {
                            let alias = t
                                .get("stable_alias")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            let tid = t
                                .get("type_id")
                                .and_then(Value::as_str)
                                .unwrap_or_default();
                            alias == type_alias || tid == type_alias || tid == type_id_str
                        })
                    });
                if let Some(type_def) = type_def {
                    let required: Vec<String> = type_def
                        .get("fields")
                        .and_then(Value::as_array)
                        .map(|fs| {
                            fs.iter()
                                .filter(|f| {
                                    f.get("required")
                                        .and_then(Value::as_bool)
                                        .unwrap_or(false)
                                })
                                .filter_map(|f| {
                                    f.get("alias")
                                        .and_then(Value::as_str)
                                        .map(str::to_string)
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    if !required.is_empty() {
                        let req_preds =
                            resolve_predicates(&mut tx, tenant, &required).await?;
                        let mut missing: Vec<String> = Vec::new();
                        for alias in &required {
                            let Some((pid, _)) = req_preds.get(alias) else {
                                continue;
                            };
                            let present = objects.iter().any(|(opid, obj)| {
                                opid == pid
                                    && !matches!(
                                        obj,
                                        StatementObject::Scalar(
                                            CanonicalScalar::Text { value_nfc, .. }
                                        ) if value_nfc.trim().is_empty()
                                    )
                            });
                            if !present {
                                missing.push(alias.clone());
                            }
                        }
                        if !missing.is_empty() {
                            return Err(http_err(
                                422,
                                &format!(
                                    "required fields missing: {}",
                                    missing.join(", ")
                                ),
                            ));
                        }
                    }
                }
            }

            // Dedup via identity policies.
            let policies: Vec<(uuid::Uuid, Vec<uuid::Uuid>)> = sqlx::query_as(
                "SELECT policy_id, key_component_predicate_ids FROM meta.identity_policy
                 WHERE tenant_id=$1 AND type_id=$2",
            )
            .bind(tenant)
            .bind(type_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;

            let mut keys: Vec<(uuid::Uuid, String)> = Vec::new();
            for (policy_id, components) in &policies {
                let mut parts = Vec::with_capacity(components.len());
                let mut complete = true;
                for comp in components {
                    match objects.iter().find(|(pid, _)| pid == comp) {
                        Some((_, StatementObject::Scalar(s))) => {
                            match scalar_key_component(s) {
                                Some(part) => parts.push(part),
                                None => {
                                    complete = false;
                                    break;
                                }
                            }
                        }
                        _ => {
                            complete = false;
                            break;
                        }
                    }
                }
                if complete && !components.is_empty() {
                    keys.push((*policy_id, enginer_domain::compute_identity_key(&parts)));
                }
            }

            if identity_mode != "create_candidate" {
                for (policy_id, key) in &keys {
                    let found: Option<(i64, uuid::Uuid, i64)> = sqlx::query_as(
                        "SELECT k.entity_id, e.public_id, e.head_revision
                         FROM core.identity_key k
                         JOIN core.entity e
                           ON e.tenant_id=k.tenant_id AND e.entity_id=k.entity_id
                         WHERE k.tenant_id=$1 AND k.type_id=$2 AND k.policy_id=$3 AND k.key=$4
                           AND k.status=0 AND e.retired_at IS NULL",
                    )
                    .bind(tenant)
                    .bind(type_id)
                    .bind(policy_id)
                    .bind(key)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    if let Some((_, public, rev)) = found {
                        if identity_mode == "reject" {
                            return Err(http_err(409, "identity collision"));
                        }
                        return ok_json(json!({
                            "public_id": public,
                            "entity_id": 0,
                            "revision": rev,
                            "outcome": "deduplicated",
                        }));
                    }
                }
            }

            let entity_id = next_val(&mut tx, "core.entity_id_seq").await?;
            let public_id = uuid::Uuid::now_v7();
            let now = chrono::Utc::now();
            sqlx::query(
                "INSERT INTO core.entity
                 (tenant_id, entity_id, public_id, type_id, entity_class, created_at, created_by, head_revision)
                 VALUES ($1,$2,$3,$4,0,$5,$6,1)",
            )
            .bind(tenant)
            .bind(entity_id)
            .bind(public_id)
            .bind(type_id)
            .bind(now)
            .bind(actor)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;

            for (pid, obj) in &objects {
                let sid = next_val(&mut tx, "core.statement_id_seq").await?;
                insert_statement(&mut tx, tenant, sid, entity_id, *pid, obj, now, actor, 1).await?;
            }
            for (policy_id, key) in &keys {
                sqlx::query(
                    "INSERT INTO core.identity_key (tenant_id, type_id, policy_id, key, entity_id, status)
                     VALUES ($1,$2,$3,$4,$5,0) ON CONFLICT DO NOTHING",
                )
                .bind(tenant)
                .bind(type_id)
                .bind(policy_id)
                .bind(key)
                .bind(entity_id)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
            sqlx::query(
                "INSERT INTO core.entity_revision
                 (tenant_id, entity_id, revision, schema_revision_id, actor, command)
                 VALUES ($1,$2,1,$3,$4,$5)",
            )
            .bind(tenant)
            .bind(entity_id)
            .bind(published)
            .bind(actor)
            .bind(serde_json::json!({"op":"create","changes":objects.iter().map(|(p,_)| p.to_string()).collect::<Vec<_>>()}).to_string())
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            outbox_event(
                &mut tx,
                tenant,
                &public_id.to_string(),
                1,
                "entity.created",
                json!({ "public_id": public_id, "type_id": type_id, "type_alias": type_alias }),
            )
            .await?;
            tx.commit().await.map_err(db_err)?;
            ok_json(json!({
                "public_id": public_id,
                "entity_id": entity_id,
                "revision": 1,
                "outcome": "created",
            }))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `PUT /entities/{public_id}` — replace field values (optimistic revision).
#[put("/entities/<public_id>", data = "<body>")]
pub async fn pg_update_entity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let body = body.into_inner();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.update").await?;
            let tenant = ctx.tenant_id;
            let actor = ctx.principal_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            let row: Option<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
                .bind(tenant)
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
            let erow = row_to_entity_row(row.ok_or_else(|| http_err(404, "entity not found"))?);
            let entity_id = erow.entity_id;
            let head_rev = erow.head_revision;
            let retired = erow.retired_at;
            let type_id = erow.type_id;
            let tkeys = type_keys(&erow);
            let tkey_refs: [&str; 2] = [&tkeys[0], &tkeys[1]];
            // Data-access checks: type update + instance update (1.1/1.3).
            if !perm.can_type(&tkey_refs, crate::dataperm::Op::Update)
                || !perm.can_entity(&pid.to_string(), crate::dataperm::Op::Update)
            {
                return Err(http_err(403, "no update access"));
            }
            if retired.is_some() {
                return Err(http_err(409, "entity retired"));
            }
            if let Some(expected) = body.get("revision").and_then(Value::as_i64) {
                if expected != head_rev {
                    return Err(http_err(409, "stale revision"));
                }
            }
            let fields = body
                .get("fields")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let aliases: Vec<String> = fields
                .iter()
                .filter_map(|s| {
                    s.get("predicate_id")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                })
                .collect();
            let preds = resolve_predicates(&mut tx, tenant, &aliases).await?;
            let now = chrono::Utc::now();
            let mut changed: Vec<String> = Vec::new();
            for stmt in &fields {
                let alias = stmt
                    .get("predicate_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let input = stmt.get("value").cloned().unwrap_or(Value::Null);
                let (pred, is_edge) = preds
                    .get(alias)
                    .copied()
                    .ok_or_else(|| http_err(422, "unknown predicate"))?;
                // Field/relation update permission per touched predicate.
                let allowed = if is_edge {
                    perm.can_relation_dir(alias, crate::dataperm::Op::Update, Some(true))
                } else {
                    perm.can_field(&tkey_refs, alias, crate::dataperm::Op::Update)
                };
                if !allowed {
                    return Err(http_err(403, "no update access to field"));
                }
                let obj = input_to_object(&mut tx, tenant, is_edge, &input).await?;
                let obj_json = serde_json::to_value(&obj).map_err(db_err)?;
                // Supersede any active statements on this predicate.
                sqlx::query(
                    "UPDATE core.statement SET status='superseded'
                     WHERE tenant_id=$1 AND subject=$2 AND predicate=$3 AND status='active'",
                )
                .bind(tenant)
                .bind(entity_id)
                .bind(pred)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                let sid = next_val(&mut tx, "core.statement_id_seq").await?;
                insert_statement(&mut tx, tenant, sid, entity_id, pred, &obj, now, actor, 1)
                    .await?;
                changed.push(format!("{alias}:{obj_json}"));
            }
            if changed.is_empty() {
                return ok_json(json!({ "public_id": pid, "revision": head_rev }));
            }
            recompute_identity_keys(&mut tx, tenant, entity_id, type_id).await?;
            let rev = bump_revision(
                &mut tx,
                tenant,
                entity_id,
                actor,
                &serde_json::json!({"op":"update","changes":changed}).to_string(),
                None,
            )
            .await?;
            outbox_event(
                &mut tx,
                tenant,
                &pid.to_string(),
                rev,
                "entity.updated",
                json!({ "public_id": pid }),
            )
            .await?;
            let row: (
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            ) = sqlx::query_as(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
                .bind(tenant)
                .bind(pid)
                .fetch_one(&mut *tx)
                .await
                .map_err(db_err)?;
            let records = record_for(&mut tx, tenant, &[row_to_entity_row(row)], &perm).await?;
            tx.commit().await.map_err(db_err)?;
            ok_json(serde_json::to_value(&records[0]).unwrap_or(Value::Null))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `POST /entities/{public_id}/retire`.
#[post("/entities/<public_id>/retire", data = "<body>")]
pub async fn pg_retire_entity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("retired")
        .to_string();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.retire").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            let row: Option<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(&format!("{ENTITY_SELECT} AND e.public_id = $2"))
                .bind(tenant)
                .bind(pid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
            let erow = row_to_entity_row(row.ok_or_else(|| http_err(404, "entity not found"))?);
            let entity_id = erow.entity_id;
            let retired = erow.retired_at;
            let tkeys = type_keys(&erow);
            if !perm.can_type(&[&tkeys[0], &tkeys[1]], crate::dataperm::Op::Delete)
                || !perm.can_entity(&pid.to_string(), crate::dataperm::Op::Delete)
            {
                return Err(http_err(403, "no delete access"));
            }
            if retired.is_some() {
                return Err(http_err(409, "already retired"));
            }
            let rev = bump_revision(
                &mut tx,
                tenant,
                entity_id,
                ctx.principal_id,
                &serde_json::json!({"op":"retire","reason":reason}).to_string(),
                None,
            )
            .await?;
            sqlx::query(
                "UPDATE core.entity SET retired_at=now(), retired_by=$3, retire_reason=$4
                 WHERE tenant_id=$1 AND entity_id=$2",
            )
            .bind(tenant)
            .bind(entity_id)
            .bind(ctx.principal_id)
            .bind(&reason)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "UPDATE core.statement SET status='revoked'
                 WHERE tenant_id=$1 AND subject=$2 AND status='active'",
            )
            .bind(tenant)
            .bind(entity_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "UPDATE core.identity_key SET status=1 WHERE tenant_id=$1 AND entity_id=$2",
            )
            .bind(tenant)
            .bind(entity_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            outbox_event(
                &mut tx,
                tenant,
                &pid.to_string(),
                rev,
                "entity.retired",
                json!({ "public_id": pid, "reason": reason }),
            )
            .await?;
            tx.commit().await.map_err(db_err)?;
            ok_json(json!({ "public_id": pid, "revision": rev, "status": "retired" }))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `POST /entities/{public_id}/merge` — loser merges into this canonical.
#[post("/entities/<public_id>/merge", data = "<body>")]
pub async fn pg_merge_entity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let canonical: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let loser: uuid::Uuid = body
        .get("loser_public_id")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .ok_or(Status::UnprocessableEntity)?;
    let reason = body
        .get("reason")
        .and_then(Value::as_str)
        .unwrap_or("merge")
        .to_string();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.merge").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            if !perm.can_entity(&canonical.to_string(), crate::dataperm::Op::Update)
                || !perm.can_entity(&loser.to_string(), crate::dataperm::Op::Delete)
            {
                return Err(http_err(403, "no merge access"));
            }
            let can: Option<(i64, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                "SELECT entity_id, retired_at FROM core.entity WHERE tenant_id=$1 AND public_id=$2",
            )
            .bind(tenant)
            .bind(canonical)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let los: Option<(i64, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
                "SELECT entity_id, retired_at FROM core.entity WHERE tenant_id=$1 AND public_id=$2",
            )
            .bind(tenant)
            .bind(loser)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (can_id, can_ret) = can.ok_or_else(|| http_err(404, "canonical not found"))?;
            let (los_id, los_ret) = los.ok_or_else(|| http_err(404, "loser not found"))?;
            if can_id == los_id {
                return Err(http_err(422, "cannot merge entity into itself"));
            }
            if can_ret.is_some() {
                return Err(http_err(409, "canonical is retired"));
            }
            if los_ret.is_some() {
                return ok_json(json!({
                    "canonical_public_id": canonical,
                    "loser_public_id": loser,
                    "canonical_revision": 0,
                    "idempotent": true,
                }));
            }
            // Re-point loser statements to canonical.
            sqlx::query(
                "UPDATE core.statement SET subject=$3
                 WHERE tenant_id=$1 AND subject=$2",
            )
            .bind(tenant)
            .bind(los_id)
            .bind(can_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            // Inbound edges to loser re-point to canonical.
            sqlx::query(
                "UPDATE core.statement SET object = jsonb_build_object('object_kind','entity','entity_id',$3::bigint)
                 WHERE tenant_id=$1 AND object_kind='entity' AND object->>'entity_id' = $2::text",
            )
            .bind(tenant)
            .bind(los_id)
            .bind(can_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            let rev = bump_revision(
                &mut tx,
                tenant,
                can_id,
                ctx.principal_id,
                &serde_json::json!({"op":"merge","loser":loser,"reason":reason}).to_string(),
                None,
            )
            .await?;
            sqlx::query(
                "UPDATE core.entity SET retired_at=now(), retired_by=$3, retire_reason=$4,
                        canonical_entity_id=$5, head_revision=head_revision+1
                 WHERE tenant_id=$1 AND entity_id=$2",
            )
            .bind(tenant)
            .bind(los_id)
            .bind(ctx.principal_id)
            .bind(&reason)
            .bind(can_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "INSERT INTO core.entity_alias (tenant_id, alias_kind, alias_value, canonical_entity_id, reason)
                 VALUES ($1,'public_id',$2,$3,$4) ON CONFLICT DO NOTHING",
            )
            .bind(tenant)
            .bind(loser.to_string())
            .bind(can_id)
            .bind(&reason)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "UPDATE core.identity_key SET entity_id=$3
                 WHERE tenant_id=$1 AND entity_id=$2",
            )
            .bind(tenant)
            .bind(los_id)
            .bind(can_id)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            outbox_event(
                &mut tx,
                tenant,
                &canonical.to_string(),
                rev,
                "entity.merged",
                json!({ "canonical": canonical, "loser": loser, "reason": reason }),
            )
            .await?;
            tx.commit().await.map_err(db_err)?;
            ok_json(json!({
                "canonical_public_id": canonical,
                "loser_public_id": loser,
                "canonical_revision": rev,
                "idempotent": false,
            }))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// --- Relations (edge statements) ---

/// Accept a full RFC3339 timestamp or a bare `YYYY-MM-DD` date (what an
/// `input[type=date]` produces). A bare date means "as of that day" —
/// interpreted as the end of the day UTC so an `as_of` of *today* still
/// sees relations created earlier today.
fn parse_as_of(s: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    if let Ok(dt) = s.parse::<chrono::DateTime<chrono::Utc>>() {
        return Some(dt);
    }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .ok()?
        .and_hms_nano_opt(23, 59, 59, 999_999_999)
        .map(|dt| dt.and_utc())
}

#[get("/entities/<public_id>/relations?<direction>&<as_of>")]
pub async fn pg_get_relations(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    direction: Option<String>,
    as_of: Option<String>,
) -> Result<Json<ApiEnvelope<Vec<RelationRecord>>>, Status> {
    let as_of = as_of
        .map(|s| parse_as_of(&s).ok_or(Status::UnprocessableEntity))
        .transpose()?;
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.relations").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            let subject: Option<(i64,)> = sqlx::query_as(
                "SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2",
            )
            .bind(tenant)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (eid,) = subject.ok_or_else(|| http_err(404, "entity not found"))?;
            // Edges touching this entity; `te` is always the "other" side.
            // `as_of` narrows to relations whose validity window covers t.
            let as_of_clause = if as_of.is_some() {
                " AND s.valid_from <= $3 AND (s.valid_to IS NULL OR s.valid_to > $3)"
            } else {
                ""
            };
            let rows: Vec<(
                i64,
                String,
                i64,
                i64,
                uuid::Uuid,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(&format!(
                "SELECT s.statement_id, pd.stable_alias, s.subject,
                        te.entity_id AS other_id,
                        te.public_id AS other_public,
                        COALESCE(td.stable_alias, te.type_id::text) AS other_type,
                        s.valid_from, s.valid_to
                 FROM core.statement s
                 JOIN meta.predicate_definition pd
                   ON pd.tenant_id=s.tenant_id AND pd.predicate_id=s.predicate
                 JOIN core.entity te
                   ON te.tenant_id=s.tenant_id
                  AND te.entity_id = CASE WHEN s.subject=$2
                                          THEN (s.object->>'entity_id')::bigint
                                          ELSE s.subject END
                 LEFT JOIN meta.type_definition td
                   ON td.tenant_id=te.tenant_id AND td.type_id=te.type_id
                 WHERE s.tenant_id=$1 AND s.object_kind='entity' AND s.status='active'
                   AND (s.subject=$2 OR (s.object->>'entity_id')::bigint=$2){as_of_clause}"
            ))
            .bind(tenant)
            .bind(eid)
            .bind(as_of.unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH))
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;

            let others: Vec<i64> = rows.iter().map(|r| r.3).collect();
            let mut fields_map = load_fields(&mut tx, tenant, &others).await?;
            let mut records = Vec::with_capacity(rows.len());
            for (sid, alias, subj, other, other_public, other_type, vf, vt) in rows {
                let outgoing = subj == eid;
                if let Some(dir) = direction.as_deref() {
                    if dir == "outgoing" && !outgoing {
                        continue;
                    }
                    if dir == "incoming" && outgoing {
                        continue;
                    }
                }
                // Relation type read + instance read + hidden-object rule (1.3-1.5).
                if !perm.can_relation_dir(&alias, crate::dataperm::Op::Read, Some(outgoing))
                    || !perm.can_relation_instance(&sid.to_string(), crate::dataperm::Op::Read)
                    || !perm.can_entity(&other_public.to_string(), crate::dataperm::Op::Read)
                {
                    continue;
                }
                let mut f = fields_map.remove(&other).unwrap_or_default();
                perm.filter_fields(&[&other_type], &mut f);
                perm.mask_fields(&[&other_type], &mut f);
                let (target_name, target_type) =
                    if perm.can_type(&[&other_type], crate::dataperm::Op::Read) {
                        (display_name_of(&other_type, &f, other), other_type)
                    } else {
                        // 1.6: relation visible but the target type is not —
                        // the peer renders as a restricted stub.
                        ("restricted".to_string(), "restricted".to_string())
                    };
                records.push(RelationRecord {
                    relation_id: sid.to_string(),
                    predicate: alias.clone(),
                    direction: if outgoing {
                        "outgoing".to_string()
                    } else {
                        "incoming".to_string()
                    },
                    target_entity_id: other_public.to_string(),
                    target_display_name: target_name,
                    target_type,
                    valid_from: vf.to_rfc3339(),
                    valid_to: vt.map(|t| t.to_rfc3339()),
                    source: "manual".to_string(),
                    qualifiers: json!({}),
                });
            }
            ok_json(serde_json::to_value(records).unwrap_or(Value::Null))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

#[post("/entities/<public_id>/relations", data = "<body>")]
pub async fn pg_create_relation(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let parsed: Value = serde_json::from_str(&body).map_err(|_| Status::BadRequest)?;
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.relation.create").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            let subject: Option<(i64,)> = sqlx::query_as(
                "SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2 AND retired_at IS NULL",
            )
            .bind(tenant)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (eid,) = subject.ok_or_else(|| http_err(404, "entity not found"))?;
            let alias = parsed
                .get("predicate")
                .or_else(|| parsed.get("predicate_id"))
                .and_then(Value::as_str)
                .ok_or_else(|| http_err(422, "predicate required"))?;
            let preds = resolve_predicates(&mut tx, tenant, &[alias.to_string()]).await?;
            let (pred, is_edge) = preds
                .get(alias)
                .copied()
                .ok_or_else(|| http_err(422, "unknown predicate"))?;
            if !is_edge {
                return Err(http_err(422, "predicate is not an edge"));
            }
            if !perm.can_relation_dir(alias, crate::dataperm::Op::Create, Some(true)) {
                return Err(http_err(403, "no create access to relation type"));
            }
            if !perm.can_entity(&pid.to_string(), crate::dataperm::Op::Update) {
                return Err(http_err(403, "no update access to entity"));
            }
            let target_public: uuid::Uuid = parsed
                .get("target_entity_id")
                .or_else(|| parsed.get("target_public_id"))
                .and_then(Value::as_str)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| http_err(422, "target_entity_id required"))?;
            let target: Option<(i64,)> = sqlx::query_as(
                "SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2 AND retired_at IS NULL",
            )
            .bind(tenant)
            .bind(target_public)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (tid,) = target.ok_or_else(|| http_err(404, "target not found"))?;
            let dup: Option<(i64,)> = sqlx::query_as(
                "SELECT statement_id FROM core.statement
                 WHERE tenant_id=$1 AND subject=$2 AND predicate=$3
                   AND object_kind='entity' AND object->>'entity_id' = $4::text
                   AND status='active'",
            )
            .bind(tenant)
            .bind(eid)
            .bind(pred)
            .bind(tid.to_string())
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            if dup.is_some() {
                return Err(http_err(409, "relation exists"));
            }
            let sid = next_val(&mut tx, "core.statement_id_seq").await?;
            insert_statement(
                &mut tx,
                tenant,
                sid,
                eid,
                pred,
                &StatementObject::Entity(enginer_domain::EntityId(tid)),
                chrono::Utc::now(),
                ctx.principal_id,
                1,
            )
            .await?;
            bump_revision(
                &mut tx,
                tenant,
                eid,
                ctx.principal_id,
                &serde_json::json!({"op":"relation.add","predicate":alias,"target":target_public}).to_string(),
                None,
            )
            .await?;
            tx.commit().await.map_err(db_err)?;
            ok_json(json!({ "relation_id": sid.to_string(), "predicate": alias }))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[delete("/entities/<public_id>/relations/<relation_id>")]
pub async fn pg_delete_relation(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    relation_id: String,
) -> Result<Json<ApiEnvelope<Value>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let sid: i64 = relation_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.relation.delete").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            // Resolve the relation first: instance delete + predicate delete.
            let rel: Option<(String,)> = sqlx::query_as(
                "SELECT pd.stable_alias
                 FROM core.statement s
                 JOIN meta.predicate_definition pd
                   ON pd.tenant_id=s.tenant_id AND pd.predicate_id=s.predicate
                 JOIN core.entity e
                   ON e.tenant_id=s.tenant_id AND e.entity_id=s.subject
                  AND e.public_id=$3
                 WHERE s.tenant_id=$1 AND s.statement_id=$2 AND s.status='active'",
            )
            .bind(tenant)
            .bind(sid)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let Some((rel_alias,)) = rel else {
                return Err(http_err(404, "relation not found"));
            };
            if !perm.can_relation_instance(&sid.to_string(), crate::dataperm::Op::Delete)
                || !perm.can_relation_dir(&rel_alias, crate::dataperm::Op::Delete, Some(true))
            {
                return Err(http_err(403, "no delete access to relation"));
            }
            let res = sqlx::query(
                "UPDATE core.statement s SET status='revoked'
                 FROM core.entity e
                 WHERE s.tenant_id=$1 AND s.statement_id=$2 AND s.status='active'
                   AND e.tenant_id=s.tenant_id AND e.entity_id=s.subject
                   AND e.public_id=$3",
            )
            .bind(tenant)
            .bind(sid)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            if res.rows_affected() == 0 {
                return Err(http_err(404, "relation not found"));
            }
            tx.commit().await.map_err(db_err)?;
            ok_json(json!({ "deleted": true }))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// --- History ---

#[get("/entities/<public_id>/history")]
pub async fn pg_get_entity_history(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
) -> Result<Json<ApiEnvelope<Vec<HistoryEntryRecord>>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.history").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            if !perm.can_entity(&pid.to_string(), crate::dataperm::Op::Read) {
                return Err(http_err(404, "entity not found"));
            }
            let entity: Option<(i64,)> = sqlx::query_as(
                "SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2",
            )
            .bind(tenant)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (eid,) = entity.ok_or_else(|| http_err(404, "entity not found"))?;
            let rows: Vec<(
                i64,
                Option<uuid::Uuid>,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<String>,
            )> = sqlx::query_as(
                "SELECT r.revision, r.actor, r.command, r.occurred_at, p.display_name
                 FROM core.entity_revision r
                 LEFT JOIN sec.principal p ON p.principal_id = r.actor
                 WHERE r.tenant_id=$1 AND r.entity_id=$2
                 ORDER BY r.revision",
            )
            .bind(tenant)
            .bind(eid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let entries: Vec<HistoryEntryRecord> = rows
                .into_iter()
                .map(|(rev, _actor, command, at, name)| {
                    let parsed = serde_json::from_str::<Value>(&command).unwrap_or_default();
                    let op = parsed
                        .get("op")
                        .and_then(Value::as_str)
                        .unwrap_or("update")
                        .to_string();
                    let changes: Vec<HistoryChangeRecord> = parsed
                        .get("changes")
                        .and_then(Value::as_array)
                        .map(|arr| {
                            arr.iter()
                                .map(|c| HistoryChangeRecord {
                                    predicate_id: c
                                        .as_str()
                                        .map(str::to_string)
                                        .or_else(|| {
                                            c.get("predicate_id")
                                                .and_then(Value::as_str)
                                                .map(str::to_string)
                                        })
                                        .unwrap_or_default(),
                                    alias: c
                                        .get("alias")
                                        .and_then(Value::as_str)
                                        .map(str::to_string)
                                        .or_else(|| c.as_str().map(str::to_string))
                                        .unwrap_or_default(),
                                    old_value: c
                                        .get("old_value")
                                        .and_then(Value::as_str)
                                        .map(str::to_string),
                                    new_value: c
                                        .get("new_value")
                                        .and_then(Value::as_str)
                                        .map(str::to_string),
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    HistoryEntryRecord {
                        revision: rev,
                        timestamp: at.to_rfc3339(),
                        actor: name.unwrap_or_else(|| "system".to_string()),
                        operation: op,
                        reason: parsed
                            .get("reason")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        changes,
                    }
                })
                .collect();
            ok_json(serde_json::to_value(entries).unwrap_or(Value::Null))
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

// --- Graph ---

#[post("/entities/<public_id>/graph", data = "<body>")]
pub async fn pg_get_entity_graph(
    cap: CapabilityGuard,
    state: &State<AppState>,
    public_id: String,
    body: String,
) -> Result<Json<ApiEnvelope<GraphDataRecord>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let req = serde_json::from_str::<Value>(&body).unwrap_or_default();
    let max_depth = req
        .get("max_depth")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 5) as usize;
    let direction = req
        .get("direction")
        .and_then(Value::as_str)
        .unwrap_or("both")
        .to_string();
    let locale = req
        .get("locale")
        .and_then(Value::as_str)
        .unwrap_or("ru")
        .to_string();
    let as_of = req
        .get("as_of")
        .and_then(Value::as_str)
        .map(|s| parse_as_of(s).ok_or(Status::UnprocessableEntity))
        .transpose()?;
    let pid: uuid::Uuid = public_id.parse().map_err(|_| Status::NotFound)?;
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.graph").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            if !perm.can_entity(&pid.to_string(), crate::dataperm::Op::Read) {
                return Err(http_err(404, "entity not found"));
            }
            let root: Option<(i64,)> = sqlx::query_as(
                "SELECT entity_id FROM core.entity WHERE tenant_id=$1 AND public_id=$2",
            )
            .bind(tenant)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let (root_id,) = root.ok_or_else(|| http_err(404, "entity not found"))?;

            // Localized predicate labels (docs/10 §4): requested locale first,
            // then ru/en fallbacks, then the stable alias.
            let label_rows: Vec<(String, String, String)> = sqlx::query_as(
                "SELECT pd.stable_alias, lt.locale, lt.text
                 FROM meta.predicate_definition pd
                 JOIN meta.localized_text lt
                   ON lt.tenant_id=pd.tenant_id AND lt.resource_kind='predicate'
                  AND lt.resource_id=pd.predicate_id AND lt.field='label'
                 WHERE pd.tenant_id=$1",
            )
            .bind(tenant)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let mut label_map: HashMap<String, String> = HashMap::new();
            for (alias, loc, text) in label_rows {
                let priority = |l: &str| match l {
                    l if l == locale => 0,
                    "ru" => 1,
                    "en" => 2,
                    _ => 3,
                };
                let replace = label_map
                    .get(&alias)
                    .map(|cur| {
                        let cur_loc = cur.split('\u{1}').next().unwrap_or_default();
                        priority(&loc) < priority(cur_loc)
                    })
                    .unwrap_or(true);
                if replace {
                    label_map.insert(alias, format!("{loc}\u{1}{text}"));
                }
            }
            let label_of = |alias: &str| -> String {
                label_map
                    .get(alias)
                    .map(|v| v.split('\u{1}').nth(1).unwrap_or(alias).to_string())
                    .unwrap_or_else(|| alias.to_string())
            };

            // All active edges (bounded) — BFS in memory like the UI expects.
            // `as_of` narrows to edges whose validity window covers t.
            let as_of_clause = if as_of.is_some() {
                " AND s.valid_from <= $2 AND (s.valid_to IS NULL OR s.valid_to > $2)"
            } else {
                ""
            };
            let edges: Vec<(i64, String, i64, i64)> = sqlx::query_as(&format!(
                "SELECT s.statement_id, pd.stable_alias, s.subject,
                        (s.object->>'entity_id')::bigint
                 FROM core.statement s
                 JOIN meta.predicate_definition pd
                   ON pd.tenant_id=s.tenant_id AND pd.predicate_id=s.predicate
                 WHERE s.tenant_id=$1 AND s.object_kind='entity' AND s.status='active'{as_of_clause}
                 LIMIT 5000"
            ))
            .bind(tenant)
            .bind(as_of.unwrap_or(chrono::DateTime::<chrono::Utc>::UNIX_EPOCH))
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;

            let mut discovered: HashSet<i64> = HashSet::from([root_id]);
            let mut frontier = vec![root_id];
            let mut edge_out: Vec<GraphEdgeRecord> = Vec::new();
            let mut seen_edges: HashSet<String> = HashSet::new();
            // Denied relation instances never reach BFS (1.5); predicate
            // direction is checked per-traversal below (1.4 + direction).
            let edges: Vec<(i64, String, i64, i64)> = edges
                .into_iter()
                .filter(|(sid, _, _, _)| {
                    perm.can_relation_instance(&sid.to_string(), crate::dataperm::Op::Read)
                })
                .collect();
            for _ in 0..max_depth {
                let mut next = Vec::new();
                for node in &frontier {
                    for (sid, alias, subj, tgt) in &edges {
                        let traversable = match direction.as_str() {
                            "outgoing" => subj == node,
                            "incoming" => tgt == node,
                            _ => subj == node || tgt == node,
                        };
                        if !traversable {
                            continue;
                        }
                        // Direction is relative to the node we expand from.
                        if !perm.can_relation_dir(
                            alias,
                            crate::dataperm::Op::Read,
                            Some(subj == node),
                        ) {
                            continue;
                        }
                        let neighbour = if subj == node { *tgt } else { *subj };
                        let cycle = discovered.contains(&neighbour);
                        if discovered.insert(neighbour) {
                            next.push(neighbour);
                        }
                        let eid = format!("{subj}:{alias}:{tgt}");
                        if seen_edges.insert(eid.clone()) {
                            edge_out.push(GraphEdgeRecord {
                                id: format!("{subj}:{alias}:{tgt}"),
                                source: String::new(),
                                target: String::new(),
                                predicate: alias.clone(),
                                label: label_of(&alias),
                                cycle,
                            });
                            let _ = sid;
                        }
                    }
                }
                frontier = next;
                if frontier.is_empty() || discovered.len() >= 500 {
                    break;
                }
            }

            // Resolve nodes: public_id + display name + type.
            let ids: Vec<i64> = discovered.iter().copied().collect();
            let ent_rows: Vec<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(
                "SELECT e.entity_id, e.public_id, e.type_id,
                        COALESCE(td.stable_alias, e.type_id::text), e.retired_at
                 FROM core.entity e
                 LEFT JOIN meta.type_definition td
                   ON td.tenant_id=e.tenant_id AND td.type_id=e.type_id
                 WHERE e.tenant_id=$1 AND e.entity_id = ANY($2)",
            )
            .bind(tenant)
            .bind(&ids)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let mut fields_map = load_fields(&mut tx, tenant, &ids).await?;
            let mut id_to_public: HashMap<i64, String> = HashMap::new();
            let mut nodes = Vec::new();
            let mut root_visible = false;
            for (eid, public, tid, alias, retired) in ent_rows {
                // Hidden entities never appear in the graph — their edges drop
                // below when endpoint ids fail to resolve (1.3).
                let type_readable =
                    perm.can_type(&[&tid.to_string(), &alias], crate::dataperm::Op::Read);
                if !type_readable
                    || !perm.can_entity(&public.to_string(), crate::dataperm::Op::Read)
                {
                    continue;
                }
                if eid == root_id {
                    root_visible = true;
                }
                let mut fields = fields_map.remove(&eid).unwrap_or_default();
                perm.filter_fields(&[&tid.to_string(), &alias], &mut fields);
                perm.mask_fields(&[&tid.to_string(), &alias], &mut fields);
                id_to_public.insert(eid, public.to_string());
                nodes.push(GraphNodeRecord {
                    id: public.to_string(),
                    label: display_name_of(&alias, &fields, eid),
                    type_label: alias.clone(),
                    type_id: alias,
                    status: if retired.is_some() {
                        "retired".to_string()
                    } else {
                        "active".to_string()
                    },
                    has_children: edges.iter().any(|(_, _, s, t)| s == &eid || t == &eid),
                    cycle: false,
                });
            }
            for edge in &mut edge_out {
                let parts: Vec<&str> = edge.id.split(':').collect();
                if let (Some(s), Some(t)) = (
                    parts.first().and_then(|p| p.parse::<i64>().ok()),
                    parts.last().and_then(|p| p.parse::<i64>().ok()),
                ) {
                    edge.source = id_to_public.get(&s).cloned().unwrap_or_default();
                    edge.target = id_to_public.get(&t).cloned().unwrap_or_default();
                    edge.id = format!("{}:{}:{}", edge.source, edge.predicate, edge.target);
                }
            }
            edge_out.retain(|e| !e.source.is_empty() && !e.target.is_empty());
            if !root_visible {
                return Err(http_err(404, "entity not found"));
            }
            ok_json(
                serde_json::to_value(GraphDataRecord {
                    nodes,
                    edges: edge_out,
                })
                .unwrap_or(Value::Null),
            )
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

// --- POST /entities/query ---

/// Resolve a `field` reference inside a `where` comparison to a field name the
/// in-memory evaluator understands (system fields or predicate aliases).
fn where_field_name(field: &Value) -> Option<String> {
    match field {
        Value::String(s) => Some(s.clone()),
        Value::Object(_) => field
            .get("system")
            .or_else(|| field.get("predicate"))
            .or_else(|| field.get("predicate_id"))
            .and_then(Value::as_str)
            .map(str::to_string),
        _ => None,
    }
}

fn normalize_cmp(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Compare an actual field value against a literal for `eq`/`ne`/ordering ops.
/// Numeric when both sides parse as numbers, otherwise normalized strings.
fn cmp_values(actual: &str, expected: &Value) -> std::cmp::Ordering {
    let exp = match expected {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    };
    if let (Ok(a), Ok(b)) = (actual.trim().parse::<f64>(), exp.trim().parse::<f64>()) {
        return a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal);
    }
    normalize_cmp(actual).cmp(&normalize_cmp(&exp))
}

/// Evaluate a `where` AST node (`and`/`or`/`not`/`comparison`) against an
/// entity record. Unknown or empty nodes match everything (fail open for
/// filtering is unsafe for dedup — the caller validates known shapes).
fn eval_where(
    node: &Value,
    entity: &EntityRecord,
    field_text: &dyn Fn(&EntityRecord, &str) -> Option<String>,
) -> bool {
    if let Some(arr) = node.get("and").and_then(Value::as_array) {
        return arr.iter().all(|c| eval_where(c, entity, field_text));
    }
    if let Some(arr) = node.get("or").and_then(Value::as_array) {
        return arr.iter().any(|c| eval_where(c, entity, field_text));
    }
    if let Some(inner) = node.get("not") {
        return !eval_where(inner, entity, field_text);
    }
    let comp = node.get("comparison").unwrap_or(node);
    let Some(field_name) = comp.get("field").and_then(where_field_name) else {
        return true;
    };
    let op = comp.get("op").and_then(Value::as_str).unwrap_or("eq");
    let expected = comp.get("value").cloned().unwrap_or(Value::Null);
    let actual = field_text(entity, &field_name);
    match op {
        "exists" => actual.is_some_and(|v| !v.trim().is_empty()),
        "not_exists" => actual.map_or(true, |v| v.trim().is_empty()),
        "in" => expected
            .as_array()
            .map(|arr| {
                actual.is_some_and(|a| {
                    arr.iter()
                        .any(|v| cmp_values(&a, v) == std::cmp::Ordering::Equal)
                })
            })
            .unwrap_or(false),
        _ => {
            let Some(actual) = actual else {
                return matches!(op, "ne" | "not_eq");
            };
            let exp_str = match &expected {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            match op {
                "eq" => cmp_values(&actual, &expected) == std::cmp::Ordering::Equal,
                "ne" | "not_eq" => cmp_values(&actual, &expected) != std::cmp::Ordering::Equal,
                "contains" => normalize_cmp(&actual).contains(&normalize_cmp(&exp_str)),
                "prefix" => normalize_cmp(&actual).starts_with(&normalize_cmp(&exp_str)),
                "suffix" => normalize_cmp(&actual).ends_with(&normalize_cmp(&exp_str)),
                "gt" => cmp_values(&actual, &expected) == std::cmp::Ordering::Greater,
                "gte" => !matches!(cmp_values(&actual, &expected), std::cmp::Ordering::Less),
                "lt" => cmp_values(&actual, &expected) == std::cmp::Ordering::Less,
                "lte" => !matches!(cmp_values(&actual, &expected), std::cmp::Ordering::Greater),
                _ => normalize_cmp(&actual).contains(&normalize_cmp(&exp_str)),
            }
        }
    }
}

#[post("/entities/query", data = "<body>")]
pub async fn pg_query_entities_post(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: String,
) -> Result<Json<ApiEnvelope<QueryResponseRecord>>, Status> {
    let exec = exec_of(state)?;
    let ctx = cap.ctx.clone();
    let parsed = serde_json::from_str::<Value>(&body).unwrap_or_default();
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = rls_tx(&pool, &ctx, "entity.query").await?;
            let tenant = ctx.tenant_id;
            let perm = perm_for(&mut tx, &ctx).await?;
            if !parsed.is_object() {
                return Err(http_err(422, "query body must be a JSON object"));
            }
            let query = match parsed.get("query") {
                Some(q) if !q.is_object() => {
                    return Err(http_err(422, "'query' must be a JSON object"));
                }
                Some(q) => q.clone(),
                None => parsed.clone(),
            };
            let type_id = query
                .get("from")
                .and_then(|f| f.get("type_id"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let search = query
                .get("search")
                .and_then(Value::as_str)
                .map(str::to_lowercase)
                .unwrap_or_default();
            let page_size = query
                .get("page")
                .and_then(|p| p.get("size"))
                .and_then(Value::as_u64)
                .unwrap_or(50)
                .clamp(1, 500) as usize;
            let offset = query
                .get("page")
                .and_then(|p| p.get("after"))
                .and_then(Value::as_str)
                .and_then(|c| c.parse::<usize>().ok())
                .unwrap_or_default();

            let rows: Vec<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                String,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = if type_id.is_empty() {
                sqlx::query_as(&format!(
                    "{ENTITY_SELECT} AND e.retired_at IS NULL ORDER BY e.entity_id LIMIT 2000"
                ))
                .bind(tenant)
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?
            } else {
                sqlx::query_as(&format!(
                    "{ENTITY_SELECT} AND (td.stable_alias=$2 OR e.type_id::text=$2)
                       AND e.retired_at IS NULL ORDER BY e.entity_id LIMIT 2000"
                ))
                .bind(tenant)
                .bind(&type_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?
            };
            let window_truncated = rows.len() >= 2000;
            let rows: Vec<EntityRow> = rows.into_iter().map(row_to_entity_row).collect();
            let records = record_for(&mut tx, tenant, &rows, &perm).await?;

            // Field filters.
            let field_filters: Vec<(String, String)> = query
                .get("filters")
                .map(|f| {
                    let mut out = Vec::new();
                    if let Some(arr) = f.as_array() {
                        for item in arr {
                            if let (Some(fi), Some(va)) = (
                                item.get("field").and_then(Value::as_str),
                                item.get("value").and_then(Value::as_str),
                            ) {
                                out.push((fi.to_string(), va.to_lowercase()));
                            }
                        }
                    } else if let Some(obj) = f.as_object() {
                        for (k, va) in obj {
                            if let Some(s) = va.as_str() {
                                out.push((k.clone(), s.to_lowercase()));
                            }
                        }
                    }
                    out
                })
                .unwrap_or_default();

            let field_text = |e: &EntityRecord, f: &str| -> Option<String> {
                match f {
                    "public_id" => Some(e.public_id.clone()),
                    "display_name" => Some(e.display_name.clone()),
                    "type_id" => Some(e.type_id.clone()),
                    "status" => Some(e.status.clone()),
                    "revision" => Some(e.revision.to_string()),
                    "created_at" => Some(e.created_at.clone()),
                    "entity_class" => Some(e.entity_class.clone()),
                    _ => e
                        .fields
                        .as_ref()
                        .and_then(Value::as_object)
                        .and_then(|o| o.get(f))
                        .and_then(wire_text),
                }
            };

            let where_node = query.get("where").cloned();
            let mut matching: Vec<&EntityRecord> = records
                .iter()
                .filter(|e| {
                    search.is_empty()
                        || e.display_name.to_lowercase().contains(&search)
                        || e.fields
                            .as_ref()
                            .and_then(Value::as_object)
                            .is_some_and(|o| {
                                o.values()
                                    .filter_map(wire_text)
                                    .any(|s| s.to_lowercase().contains(&search))
                            })
                })
                .filter(|e| {
                    where_node
                        .as_ref()
                        .map_or(true, |w| eval_where(w, e, &field_text))
                })
                .filter(|e| {
                    field_filters.iter().all(|(f, val)| {
                        field_text(e, f).is_some_and(|a| a.to_lowercase().contains(val))
                    })
                })
                .collect();
            matching.sort_by(|a, b| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            });
            let total = matching.len();
            let rows_out: Vec<Value> = matching
                .into_iter()
                .skip(offset)
                .take(page_size)
                .map(|e| {
                    let mut row = json!({
                        "public_id": e.public_id,
                        "display_name": e.display_name,
                        "type_id": e.type_id,
                        "status": e.status,
                    });
                    if let Some(f) = &e.fields {
                        if let Some(obj) = row.as_object_mut() {
                            if let Some(fo) = f.as_object() {
                                for (k, v) in fo {
                                    obj.insert(k.clone(), v.clone());
                                }
                            }
                        }
                    }
                    row
                })
                .collect();
            let next = offset + rows_out.len();
            ok_json(
                serde_json::to_value(QueryResponseRecord {
                    rows: rows_out,
                    cursor: (next < total).then(|| next.to_string()),
                    total_estimate: Some(total as i64),
                    truncated: window_truncated,
                })
                .unwrap_or(Value::Null),
            )
        })
        .await
        .map_err(run_err)?;
    Ok(Json(ApiEnvelope {
        data: json_to(v)?,
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        pg_query_entities,
        pg_query_entities_post,
        pg_get_entity,
        pg_create_entity,
        pg_update_entity,
        pg_retire_entity,
        pg_merge_entity,
        pg_get_relations,
        pg_create_relation,
        pg_delete_relation,
        pg_get_entity_history,
        pg_get_entity_graph,
    ]
}
