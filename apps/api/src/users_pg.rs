//! User administration (1.10) + per-user activity views.
//!
//! Backed by `sec.principal` / `sec.principal_credential` /
//! `sec.principal_group_membership` / `sec.session` for identity and
//! membership data, and by `core.entity.created_by`, `core.entity_revision`
//! and `audit.event` for the activity feed.
//!
//! All routes sit behind the `users.manage` capability (see
//! `required_capability`).

use crate::audit_pg::audit_tx;
use crate::entity_pg::{db_err, http_err};
use crate::workspace_routes::{meta, ApiEnvelope, CapabilityGuard};
use crate::AppState;
use crate::ApiError;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{delete, get, post, put, State};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

fn pg(state: &State<AppState>) -> Result<Arc<AsyncStdDbExecutor>, ApiError> {
    state.db_executor.clone().ok_or_else(|| ApiError::from(Status::ServiceUnavailable))
}

fn run_err(e: &DbError) -> ApiError {
    ApiError::of_db(e)
}

// ---------------------------------------------------------------------------
// Group resolution helpers
// ---------------------------------------------------------------------------

/// All groups of the tenant with parent links (for tree rendering / filters).
#[get("/users/groups")]
pub async fn list_role_groups(
    cap: CapabilityGuard,
    state: &State<AppState>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.groups").await?;
            let rows = sqlx::query(
                "SELECT group_id, parent_id, class, alias, description
                 FROM sec.\"group\" WHERE tenant_id=$1 ORDER BY alias",
            )
            .bind(ctx.tenant_id)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let out: Vec<Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "id": r.try_get::<uuid::Uuid,_>("group_id").unwrap_or_default().to_string(),
                        "parent_id": r.try_get::<Option<uuid::Uuid>,_>("parent_id")
                            .ok().flatten().map(|u| u.to_string()),
                        "class": r.try_get::<String,_>("class").unwrap_or_default(),
                        "name": r.try_get::<String,_>("alias").unwrap_or_default(),
                        "description": r.try_get::<Option<String>,_>("description")
                            .ok().flatten().unwrap_or_default(),
                    })
                })
                .collect();
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// User list with filters
// ---------------------------------------------------------------------------

/// `GET /users?q=&group_id=&inherited=&ip=&status=`
///
/// `q`       — free text over display_name and login;
/// `group_id`— members of the group; with `inherited=1` also members of its
///             descendant roles (role tree), otherwise direct members only;
/// `ip`      — users having a session from a matching IP prefix;
/// `status`  — `active|disabled` (default: all).
#[get("/users?<q>&<group_id>&<inherited>&<ip>&<status>&<limit>")]
pub async fn list_users(
    cap: CapabilityGuard,
    state: &State<AppState>,
    q: Option<String>,
    group_id: Option<String>,
    inherited: Option<String>,
    ip: Option<String>,
    status: Option<String>,
    limit: Option<u32>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let q = q.unwrap_or_default().trim().to_lowercase();
    let ip = ip.unwrap_or_default().trim().to_string();
    let status = status.unwrap_or_default().trim().to_string();
    let with_inherited = matches!(inherited.as_deref(), Some("1") | Some("true") | Some("yes"));
    // Present-but-invalid group filter must not silently widen to an
    // unfiltered user listing.
    let group_filter: Option<uuid::Uuid> = match group_id {
        None => None,
        Some(g) => Some(
            g.parse::<uuid::Uuid>()
                .map_err(|_| ApiError::from(Status::UnprocessableEntity))?,
        ),
    };
    let limit = limit.unwrap_or(200).clamp(1, 1000) as i64;
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.list").await?;
            // Role filter resolves inside SQL: direct membership, or membership
            // in the group + any descendant via parent_id.
            let rows = sqlx::query(
                "WITH RECURSIVE role_scope AS (
                     SELECT g.group_id FROM sec.\"group\" g
                      WHERE g.tenant_id=$1 AND g.group_id=$5
                     UNION
                     SELECT c.group_id FROM sec.\"group\" c
                     JOIN role_scope rs ON c.parent_id = rs.group_id
                      WHERE c.tenant_id=$1 AND $6
                 )
                 SELECT p.principal_id, p.display_name, p.status, p.is_service_account,
                        p.last_seen_at, p.email,
                        COALESCE((SELECT c.login FROM sec.principal_credential c
                                  WHERE c.principal_id=p.principal_id
                                  ORDER BY c.updated_at DESC LIMIT 1), '') AS login,
                        COALESCE((SELECT jsonb_agg(DISTINCT g.alias ORDER BY g.alias)
                                  FROM sec.principal_group_membership m
                                  JOIN sec.\"group\" g ON g.group_id=m.group_id
                                  WHERE m.principal_id=p.principal_id AND m.tenant_id=p.tenant_id
                                    AND m.valid_to IS NULL), '[]'::jsonb) AS roles,
                        (SELECT s.ip_address FROM sec.session s
                          WHERE s.principal_id=p.principal_id
                            AND s.ip_address IS NOT NULL
                          ORDER BY s.started_at DESC LIMIT 1) AS last_ip
                 FROM sec.principal p
                 WHERE p.tenant_id=$1
                   AND ($2 = '' OR lower(p.display_name) LIKE '%'||$2||'%'
                        OR lower(COALESCE(p.email,'')) LIKE '%'||$2||'%'
                        OR EXISTS (SELECT 1 FROM sec.principal_credential c
                                   WHERE c.principal_id=p.principal_id
                                     AND lower(c.login) LIKE '%'||$2||'%'))
                   AND ($3 = '' OR p.status = $3)
                   AND ($4 = '' OR EXISTS (SELECT 1 FROM sec.session s
                                   WHERE s.principal_id=p.principal_id
                                     AND s.ip_address LIKE $4||'%'))
                   AND ($5::uuid IS NULL OR EXISTS (
                        SELECT 1 FROM sec.principal_group_membership m
                        WHERE m.principal_id=p.principal_id AND m.tenant_id=p.tenant_id
                          AND m.valid_to IS NULL
                          AND m.group_id IN (SELECT group_id FROM role_scope)))
                 ORDER BY p.display_name
                 LIMIT $7",
            )
            .bind(ctx.tenant_id)
            .bind(&q)
            .bind(&status)
            .bind(&ip)
            .bind(group_filter)
            .bind(with_inherited)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let out: Vec<Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "principal_id": r.try_get::<uuid::Uuid,_>("principal_id")
                            .unwrap_or_default().to_string(),
                        "display_name": r.try_get::<String,_>("display_name").unwrap_or_default(),
                        "login": r.try_get::<String,_>("login").unwrap_or_default(),
                        "email": r.try_get::<Option<String>,_>("email").ok().flatten(),
                        "status": r.try_get::<String,_>("status").unwrap_or_default(),
                        "is_service_account": r.try_get::<bool,_>("is_service_account")
                            .unwrap_or(false),
                        "last_seen_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("last_seen_at")
                            .ok().flatten().map(|t| t.to_rfc3339()),
                        "last_ip": r.try_get::<Option<String>,_>("last_ip").ok().flatten(),
                        "roles": r.try_get::<Value,_>("roles").unwrap_or(json!([])),
                    })
                })
                .collect();
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// User CRUD
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct UserInput {
    pub login: String,
    pub display_name: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub groups: Vec<String>, // group ids or aliases
    #[serde(default)]
    pub is_service_account: bool,
    #[serde(default)]
    pub email: Option<String>,
}

/// The value the PDP uses to detect special roles on an effective group:
/// `COALESCE(NULLIF(special,''), alias)`.
const SPECIAL_ROLE_SQL: &str =
    "COALESCE(NULLIF(g.special,''), g.alias) IN ('superadmin','superuser')";

/// Is `caller` an effective member of the `superadmin` preset role
/// (membership or role-tree inheritance)? Only such principals may assign
/// or manage special roles / special-role members — otherwise any holder
/// of `users.manage` could mint itself a superadmin.
async fn caller_is_superadmin(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: uuid::Uuid,
    caller: uuid::Uuid,
) -> Result<bool, DbError> {
    let ok: bool = sqlx::query_scalar(
        "WITH RECURSIVE eff AS (
             SELECT g.group_id, g.parent_id
             FROM sec.principal_group_membership m
             JOIN sec.\"group\" g ON g.group_id = m.group_id
             WHERE m.tenant_id=$1 AND m.principal_id=$2
               AND (m.valid_to IS NULL OR m.valid_to >= now())
               AND (m.valid_from <= now() + interval '120 seconds')
             UNION
             SELECT p.group_id, p.parent_id
             FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
             WHERE p.tenant_id=$1
         )
         SELECT EXISTS(
             SELECT 1 FROM eff e JOIN sec.\"group\" g ON g.group_id=e.group_id
             WHERE COALESCE(NULLIF(g.special,''), g.alias)='superadmin')",
    )
    .bind(tenant)
    .bind(caller)
    .fetch_one(&mut **tx)
    .await
    .map_err(db_err)?;
    Ok(ok)
}

/// Is `principal` an effective member of ANY special role
/// (superadmin/superuser)? Mutating such accounts — status, groups,
/// password reset, API tokens — is restricted to superadmins.
async fn principal_is_special(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: uuid::Uuid,
    principal: uuid::Uuid,
) -> Result<bool, DbError> {
    let flag = format!(
        "WITH RECURSIVE eff AS (
             SELECT g.group_id, g.parent_id
             FROM sec.principal_group_membership m
             JOIN sec.\"group\" g ON g.group_id = m.group_id
             WHERE m.tenant_id=$1 AND m.principal_id=$2
               AND (m.valid_to IS NULL OR m.valid_to >= now())
               AND (m.valid_from <= now() + interval '120 seconds')
             UNION
             SELECT p.group_id, p.parent_id
             FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
             WHERE p.tenant_id=$1
         )
         SELECT EXISTS(
             SELECT 1 FROM eff e JOIN sec.\"group\" g ON g.group_id=e.group_id
             WHERE {SPECIAL_ROLE_SQL})"
    );
    sqlx::query_scalar(&flag)
        .bind(tenant)
        .bind(principal)
        .fetch_one(&mut **tx)
        .await
        .map_err(db_err)
}

async fn resolve_group_ids(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: uuid::Uuid,
    groups: &[String],
) -> Result<Vec<(uuid::Uuid, bool)>, DbError> {
    // ORDER BY created_at makes alias resolution deterministic when
    // legacy duplicate-alias groups still exist (new duplicates are
    // rejected at POST/PUT time): the oldest row is the canonical one.
    let sql = format!(
        "SELECT group_id, {SPECIAL_ROLE_SQL} AS is_special
         FROM sec.\"group\" g
         WHERE tenant_id=$1 AND (group_id::text=$2 OR alias=$2)
         ORDER BY created_at LIMIT 1"
    );
    let mut out = Vec::new();
    for g in groups {
        let row: Option<(uuid::Uuid, bool)> = sqlx::query_as(&sql)
            .bind(tenant)
            .bind(g.trim())
            .fetch_optional(&mut **tx)
            .await
            .map_err(db_err)?;
        let Some(gid) = row else {
            // Unknown role must fail loudly — silently dropping it would
            // mislead the admin about the user's actual grants.
            return Err(http_err(422, "unknown group"));
        };
        out.push(gid);
    }
    Ok(out)
}

async fn set_memberships(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tenant: uuid::Uuid,
    principal: uuid::Uuid,
    groups: &[String],
    caller: uuid::Uuid,
) -> Result<(), DbError> {
    // Memberships are versioned rows: close those no longer present instead
    // of deleting, so role history stays auditable (valid_to is honored by
    // all effective-membership queries).
    let now = chrono::Utc::now();
    let resolved = resolve_group_ids(tx, tenant, groups).await?;
    // Assigning a special role is a privilege grant equivalent to becoming
    // superadmin — restrict it to effective superadmins.
    if resolved.iter().any(|(_, special)| *special)
        && !caller_is_superadmin(tx, tenant, caller).await?
    {
        return Err(http_err(403, "special roles require superadmin"));
    }
    let gids: Vec<uuid::Uuid> = resolved.iter().map(|(id, _)| *id).collect();
    sqlx::query(
        "UPDATE sec.principal_group_membership SET valid_to = $3
         WHERE tenant_id=$1 AND principal_id=$2
           AND (valid_to IS NULL OR valid_to > $3)
           AND NOT (group_id = ANY($4))",
    )
    .bind(tenant)
    .bind(principal)
    .bind(now)
    .bind(&gids)
    .execute(&mut **tx)
    .await
    .map_err(db_err)?;
    for gid in &gids {
        // valid_from uses the application clock: the PDP compares it
        // against request time (also app clock). A DB default would make
        // fresh memberships invisible under DB↔app clock skew.
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
        .bind(principal)
        .bind(gid)
        .bind(now)
        .execute(&mut **tx)
        .await
        .map_err(db_err)?;
    }
    Ok(())
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    use argon2::password_hash::{rand_core::OsRng, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2::PasswordHasher::hash_password(&argon2::Argon2::default(), password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|_| ApiError::from(Status::InternalServerError))
}

#[post("/users", data = "<body>")]
pub async fn create_user(
    cap: CapabilityGuard,
    state: &State<AppState>,
    body: Json<UserInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let body = body.into_inner();
    let login = body.login.trim().to_string();
    let name = body.display_name.trim().to_string();
    let password = body.password.clone().unwrap_or_default();
    // Service accounts may omit the password entirely — they authenticate
    // with API tokens. A human user still needs one.
    let pw_ok = if password.is_empty() {
        body.is_service_account
    } else {
        password.len() >= 8
    };
    if login.is_empty() || name.is_empty() || !pw_ok {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    // Password-less service accounts get a syntactically invalid PHC string —
    // `verify_password` can never match it, so the login cannot be used for
    // a password session while still reserving the login name.
    let hash = if password.is_empty() {
        "!".to_string()
    } else {
        hash_password(&password)?
    };
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.create").await?;
            let dup: Option<(uuid::Uuid,)> =
                sqlx::query_as("SELECT credential_id FROM sec.principal_credential WHERE login=$1")
                    .bind(&login)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(db_err)?;
            if dup.is_some() {
                return Err(http_err(409, "login already exists"));
            }
            let pid = uuid::Uuid::now_v7();
            // Present-but-invalid status must not silently create an
            // *active* account the admin meant to disable.
            let status = match body.status.as_deref().map(str::trim) {
                None => "active".to_string(),
                Some(s) if matches!(s, "active" | "disabled" | "locked") => s.to_string(),
                Some(_) => return Err(http_err(422, "status.invalid")),
            };
            let email = body
                .email
                .as_deref()
                .map(str::trim)
                .filter(|e| !e.is_empty())
                .map(|e| e.to_lowercase());
            sqlx::query(
                "INSERT INTO sec.principal
                 (principal_id, tenant_id, display_name, is_service_account, status, email)
                 VALUES ($1,$2,$3,$4,$5,$6)",
            )
            .bind(pid)
            .bind(ctx.tenant_id)
            .bind(&name)
            .bind(body.is_service_account)
            .bind(&status)
            .bind(&email)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "INSERT INTO sec.principal_credential
                 (tenant_id, principal_id, login, password_hash) VALUES ($1,$2,$3,$4)",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .bind(&login)
            .bind(&hash)
            .execute(&mut *tx)
            .await
            // TOCTOU: a racing create slips past the pre-check and hits the
            // unique index — map it to the same 409, not a generic 500.
            .map_err(|e| match &e {
                sqlx::Error::Database(dbe) if dbe.is_unique_violation() => {
                    http_err(409, "login already exists")
                }
                _ => db_err(e),
            })?;
            set_memberships(&mut tx, ctx.tenant_id, pid, &body.groups, ctx.principal_id).await?;
            tx.commit().await.map_err(db_err)?;
            Ok(json!({
                "principal_id": pid.to_string(),
                "login": login,
                "display_name": name,
                "status": status,
                "is_service_account": body.is_service_account,
            }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[get("/users/<principal_id>")]
pub async fn get_user(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.get").await?;
            let row = sqlx::query(
                "SELECT p.principal_id, p.display_name, p.status, p.is_service_account,
                        p.created_at, p.last_seen_at, p.email,
                        COALESCE((SELECT c.login FROM sec.principal_credential c
                                  WHERE c.principal_id=p.principal_id
                                  ORDER BY c.updated_at DESC LIMIT 1), '') AS login
                 FROM sec.principal p
                 WHERE p.tenant_id=$1 AND p.principal_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            let Some(row) = row else {
                return Err(http_err(404, "user not found"));
            };
            // Direct memberships + resolved effective roles (with ancestors).
            let direct: Vec<Value> = sqlx::query(
                "SELECT g.group_id, g.alias, g.class, g.parent_id
                 FROM sec.principal_group_membership m
                 JOIN sec.\"group\" g ON g.group_id=m.group_id
                 WHERE m.tenant_id=$1 AND m.principal_id=$2 AND m.valid_to IS NULL
                 ORDER BY g.alias",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?
            .iter()
            .map(|r| {
                json!({
                    "id": r.try_get::<uuid::Uuid,_>("group_id").unwrap_or_default().to_string(),
                    "name": r.try_get::<String,_>("alias").unwrap_or_default(),
                    "class": r.try_get::<String,_>("class").unwrap_or_default(),
                    "parent_id": r.try_get::<Option<uuid::Uuid>,_>("parent_id")
                        .ok().flatten().map(|u| u.to_string()),
                })
            })
            .collect();
            let effective: Vec<String> = sqlx::query_scalar(
                "WITH RECURSIVE eff AS (
                     SELECT g.group_id, g.parent_id
                     FROM sec.principal_group_membership m
                     JOIN sec.\"group\" g ON g.group_id=m.group_id
                     WHERE m.tenant_id=$1 AND m.principal_id=$2 AND m.valid_to IS NULL
                     UNION
                     SELECT p.group_id, p.parent_id
                     FROM sec.\"group\" p JOIN eff e ON e.parent_id=p.group_id
                     WHERE p.tenant_id=$1
                 )
                 SELECT DISTINCT g.alias FROM sec.\"group\" g
                 JOIN eff e ON e.group_id=g.group_id ORDER BY g.alias",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let sessions: Vec<Value> = sqlx::query(
                "SELECT session_id, ip_address, user_agent, started_at, last_seen_at,
                        expires_at, ended_at
                 FROM sec.session
                 WHERE tenant_id=$1 AND principal_id=$2
                 ORDER BY started_at DESC LIMIT 20",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?
            .iter()
            .map(|r| {
                let dt = |k: &str| {
                    r.try_get::<Option<chrono::DateTime<chrono::Utc>>, _>(k)
                        .ok()
                        .flatten()
                        .map(|t| t.to_rfc3339())
                };
                json!({
                    "session_id": r.try_get::<uuid::Uuid,_>("session_id")
                        .unwrap_or_default().to_string(),
                    "ip_address": r.try_get::<Option<String>,_>("ip_address").ok().flatten(),
                    "user_agent": r.try_get::<Option<String>,_>("user_agent").ok().flatten(),
                    "started_at": dt("started_at"),
                    "last_seen_at": dt("last_seen_at"),
                    "expires_at": dt("expires_at"),
                    "ended_at": dt("ended_at"),
                })
            })
            .collect();
            Ok(json!({
                "principal_id": pid.to_string(),
                "display_name": row.try_get::<String,_>("display_name").unwrap_or_default(),
                "login": row.try_get::<String,_>("login").unwrap_or_default(),
                "email": row.try_get::<Option<String>,_>("email").ok().flatten(),
                "status": row.try_get::<String,_>("status").unwrap_or_default(),
                "is_service_account": row.try_get::<bool,_>("is_service_account")
                    .unwrap_or(false),
                "created_at": row
                    .try_get::<chrono::DateTime<chrono::Utc>,_>("created_at")
                    .map(|t| t.to_rfc3339()).ok(),
                "last_seen_at": row
                    .try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("last_seen_at")
                    .ok().flatten().map(|t| t.to_rfc3339()),
                "roles": direct,
                "effective_roles": effective,
                "sessions": sessions,
            }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

#[put("/users/<principal_id>", data = "<body>")]
pub async fn update_user(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
    body: Json<Value>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let body = body.into_inner();
    let groups: Vec<String> = body
        .get("groups")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let has_groups = body.get("groups").is_some();
    let password = body
        .get("password")
        .and_then(Value::as_str)
        .map(str::to_string);
    let hash = match &password {
        Some(p) if !p.is_empty() => {
            if p.len() < 8 {
                return Err(ApiError::from(Status::UnprocessableEntity));
            }
            Some(hash_password(p)?)
        }
        _ => None,
    };
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.update").await?;
            let exists: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT principal_id FROM sec.principal
                 WHERE tenant_id=$1 AND principal_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            if exists.is_none() {
                return Err(http_err(404, "user not found"));
            }
            // Special-role members (superadmin/superuser) may only be
            // mutated by an effective superadmin — otherwise `users.manage`
            // becomes a privilege-escalation path.
            if principal_is_special(&mut tx, ctx.tenant_id, pid).await?
                && !caller_is_superadmin(&mut tx, ctx.tenant_id, ctx.principal_id).await?
            {
                return Err(http_err(403, "special-role accounts require superadmin"));
            }
            if let Some(name) = body.get("display_name").and_then(Value::as_str) {
                if name.trim().is_empty() {
                    return Err(http_err(422, "display_name empty"));
                }
                sqlx::query(
                    "UPDATE sec.principal SET display_name=$3
                     WHERE tenant_id=$1 AND principal_id=$2",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .bind(name.trim())
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
            if let Some(status) = body.get("status").and_then(Value::as_str) {
                if !matches!(status, "active" | "disabled" | "locked") {
                    return Err(http_err(422, "bad status"));
                }
                if status != "active" && pid == ctx.principal_id {
                    // Self-disable via PUT ends the caller's own session — same
                    // guard as DELETE /users/<self>.
                    return Err(http_err(422, "cannot disable self"));
                }
                sqlx::query(
                    "UPDATE sec.principal SET status=$3
                     WHERE tenant_id=$1 AND principal_id=$2",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .bind(status)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                if status != "active" {
                    sqlx::query(
                        "UPDATE sec.session SET ended_at=now()
                         WHERE tenant_id=$1 AND principal_id=$2 AND ended_at IS NULL",
                    )
                    .bind(ctx.tenant_id)
                    .bind(pid)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                    sqlx::query(
                        "UPDATE sec.api_token SET revoked_at=now()
                         WHERE tenant_id=$1 AND principal_id=$2 AND revoked_at IS NULL",
                    )
                    .bind(ctx.tenant_id)
                    .bind(pid)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;
                }
            }
            if body.get("email").is_some() {
                let email = body
                    .get("email")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|e| !e.is_empty())
                    .map(|e| e.to_lowercase());
                sqlx::query(
                    "UPDATE sec.principal SET email=$3
                     WHERE tenant_id=$1 AND principal_id=$2",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .bind(&email)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
            if let Some(h) = &hash {
                sqlx::query(
                    "UPDATE sec.principal_credential SET password_hash=$3,
                         failed_attempts=0, locked_until=NULL, updated_at=now()
                     WHERE tenant_id=$1 AND principal_id=$2",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .bind(h)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                // An admin reset means the old credential may be
                // compromised — kill live sessions and API tokens exactly
                // like the disable path does.
                sqlx::query(
                    "UPDATE sec.session SET ended_at=now()
                     WHERE tenant_id=$1 AND principal_id=$2 AND ended_at IS NULL",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
                sqlx::query(
                    "UPDATE sec.api_token SET revoked_at=now()
                     WHERE tenant_id=$1 AND principal_id=$2 AND revoked_at IS NULL",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .execute(&mut *tx)
                .await
                .map_err(db_err)?;
            }
            if has_groups {
                set_memberships(&mut tx, ctx.tenant_id, pid, &groups, ctx.principal_id).await?;
            }
            tx.commit().await.map_err(db_err)?;
            Ok(json!({ "principal_id": pid.to_string(), "updated": true }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `DELETE /users/<id>` — soft-delete: principal disabled, sessions ended.
/// Principals are referenced by audit history, so rows are never hard-deleted.
#[delete("/users/<principal_id>")]
pub async fn delete_user(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    if pid == ctx.principal_id {
        // Disabling your own account mid-session kills the session and locks
        // the admin out — require a different principal to do it.
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.delete").await?;
            if principal_is_special(&mut tx, ctx.tenant_id, pid).await?
                && !caller_is_superadmin(&mut tx, ctx.tenant_id, ctx.principal_id).await?
            {
                return Err(http_err(403, "special-role accounts require superadmin"));
            }
            let changed = sqlx::query(
                "UPDATE sec.principal SET status='disabled'
                 WHERE tenant_id=$1 AND principal_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected();
            if changed == 0 {
                return Err(http_err(404, "user not found"));
            }
            sqlx::query(
                "UPDATE sec.session SET ended_at=now()
                 WHERE tenant_id=$1 AND principal_id=$2 AND ended_at IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            sqlx::query(
                "UPDATE sec.principal_group_membership SET valid_to=now()
                 WHERE tenant_id=$1 AND principal_id=$2 AND valid_to IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            // Disabled principals must not keep working API credentials.
            sqlx::query(
                "UPDATE sec.api_token SET revoked_at=now()
                 WHERE tenant_id=$1 AND principal_id=$2 AND revoked_at IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?;
            tx.commit().await.map_err(db_err)?;
            Ok(json!({ "principal_id": pid.to_string(), "status": "disabled" }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// Activity: created / changed / viewed objects + action log with free search
// ---------------------------------------------------------------------------

/// `GET /users/<id>/activity?q=&limit=` — the user-detail activity view.
///
/// - `created` / `changed` / `viewed`: entities the principal created, edited
///   (revision actor), or opened (`GET /entities/<id>` audit events);
/// - `log`: raw audit trail rows;
/// - `q`: free text over action, path and details of the log.
#[get("/users/<principal_id>/activity?<q>&<limit>")]
pub async fn user_activity(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
    q: Option<String>,
    limit: Option<u32>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let needle = q.unwrap_or_default().trim().to_lowercase();
    let limit = limit.unwrap_or(50).clamp(1, 500) as i64;
    let v = exec
        .run(Deadline::in_secs(15), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.activity").await?;
            let tenant = ctx.tenant_id;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sec.principal
                               WHERE tenant_id=$1 AND principal_id=$2)",
            )
            .bind(tenant)
            .bind(pid)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
            if !exists {
                return Err(http_err(404, "user not found"));
            }
            // The feed lists entities the target principal touched —
            // each row must still be readable by *this* requester,
            // otherwise the activity view leaks entity existence and
            // types across catalog boundaries (docs/28 §4/§8).
            let perm = crate::entity_pg::perm_for(&mut tx, &ctx).await?;
            // (entity_id, public_id, type_alias, at) — entity_id is the
            // join key for the batch visibility check below.
            type FeedRow = (i64, uuid::Uuid, String, chrono::DateTime<chrono::Utc>);
            let feed_json = |r: &FeedRow| {
                json!({
                    "public_id": r.1.to_string(),
                    "type": r.2,
                    "at": r.3.to_rfc3339(),
                })
            };
            let created_rows: Vec<FeedRow> = sqlx::query_as(
                "SELECT e.entity_id, e.public_id,
                        COALESCE(td.stable_alias,'') AS type_alias,
                        e.created_at AS at
                 FROM core.entity e
                 LEFT JOIN meta.type_definition td
                   ON td.tenant_id=e.tenant_id AND td.type_id=e.type_id
                 WHERE e.tenant_id=$1 AND e.created_by=$2
                 ORDER BY e.created_at DESC LIMIT $3",
            )
            .bind(tenant)
            .bind(pid)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let changed_rows: Vec<FeedRow> = sqlx::query_as(
                "SELECT e.entity_id, e.public_id,
                        COALESCE(td.stable_alias,'') AS type_alias,
                        max(r.occurred_at) AS at
                 FROM core.entity_revision r
                 JOIN core.entity e ON e.tenant_id=r.tenant_id AND e.entity_id=r.entity_id
                 LEFT JOIN meta.type_definition td
                   ON td.tenant_id=e.tenant_id AND td.type_id=e.type_id
                 WHERE r.tenant_id=$1 AND r.actor=$2
                 GROUP BY e.entity_id, e.public_id, td.stable_alias
                 ORDER BY at DESC LIMIT $3",
            )
            .bind(tenant)
            .bind(pid)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            // Viewed: audit events logged by the AuditFairing for
            // `GET /api/v1/entities/<public_id>` requests.
            let viewed_rows: Vec<FeedRow> = sqlx::query_as(
                "SELECT e.entity_id, e.public_id,
                        COALESCE(td.stable_alias,'') AS type_alias,
                        max(a.occurred_at) AS at
                 FROM audit.event a
                 JOIN core.entity e
                   ON e.tenant_id=a.tenant_id
                  AND e.public_id::text = a.metadata->>'public_id'
                 LEFT JOIN meta.type_definition td
                   ON td.tenant_id=e.tenant_id AND td.type_id=e.type_id
                 WHERE a.tenant_id=$1 AND a.principal_id=$2
                   AND a.action LIKE 'GET /api/v1/entities/%'
                   AND a.outcome='allowed'
                 GROUP BY e.entity_id, e.public_id, td.stable_alias
                 ORDER BY at DESC LIMIT $3",
            )
            .bind(tenant)
            .bind(pid)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            // One batched visibility pass over every entity referenced
            // by the three feeds (catalog + type + instance rules).
            let mut all_ids: Vec<i64> =
                Vec::with_capacity(created_rows.len() + changed_rows.len() + viewed_rows.len());
            for r in created_rows.iter().chain(&changed_rows).chain(&viewed_rows) {
                all_ids.push(r.0);
            }
            all_ids.sort_unstable();
            all_ids.dedup();
            let visible: std::collections::HashSet<i64> = if all_ids.is_empty() {
                std::collections::HashSet::new()
            } else {
                let rows: Vec<crate::entity_pg::EntityRowTuple> = sqlx::query_as(&format!(
                    "{} AND e.entity_id = ANY($2)",
                    crate::entity_pg::ENTITY_SELECT
                ))
                .bind(tenant)
                .bind(&all_ids)
                .fetch_all(&mut *tx)
                .await
                .map_err(db_err)?;
                rows.into_iter()
                    .map(crate::entity_pg::row_to_entity_row)
                    .filter(|r| crate::entity_pg::entity_visible(&perm, r))
                    .map(|r| r.entity_id)
                    .collect()
            };
            let keep = |r: &&FeedRow| visible.contains(&r.0);
            let created: Vec<Value> = created_rows.iter().filter(keep).map(feed_json).collect();
            let changed: Vec<Value> = changed_rows.iter().filter(keep).map(feed_json).collect();
            let viewed: Vec<Value> = viewed_rows.iter().filter(keep).map(feed_json).collect();
            // Raw audit rows: same catalog gate as GET /audit — tagged
            // rows are visible only when the requester reads the catalog.
            let log: Vec<Value> = sqlx::query(
                "SELECT a.audit_id, a.kind, a.action, a.outcome, a.occurred_at,
                        COALESCE(a.metadata::text,'') AS parameters,
                        a.catalog_id,
                        cat.public_id AS catalog_pid, cat.alias AS catalog_alias
                 FROM audit.event a
                 LEFT JOIN core.catalog cat
                   ON cat.tenant_id=a.tenant_id AND cat.catalog_id=a.catalog_id
                 WHERE a.tenant_id=$1 AND a.principal_id=$2
                   AND ($3 = '' OR lower(a.action) LIKE '%'||$3||'%'
                        OR lower(a.metadata::text) LIKE '%'||$3||'%')
                 ORDER BY a.occurred_at DESC LIMIT $4",
            )
            .bind(tenant)
            .bind(pid)
            .bind(&needle)
            .bind(limit)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?
            .iter()
            .filter(|r| {
                let pid: Option<uuid::Uuid> = r.try_get("catalog_pid").ok().flatten();
                let alias: Option<String> = r.try_get("catalog_alias").ok().flatten();
                match (pid, alias) {
                    (Some(p), Some(a)) => {
                        perm.can_catalog(&[&p.to_string(), &a], crate::dataperm::Op::Read)
                    }
                    // Tagged row whose catalog vanished → hide; untagged → visible.
                    (None, None) => r
                        .try_get::<Option<i64>, _>("catalog_id")
                        .ok()
                        .flatten()
                        .is_none(),
                    _ => false,
                }
            })
            .map(|r| {
                json!({
                    "audit_id": r.try_get::<i64,_>("audit_id").unwrap_or_default(),
                    "kind": r.try_get::<String,_>("kind").unwrap_or_default(),
                    "action": r.try_get::<String,_>("action").unwrap_or_default(),
                    "outcome": r.try_get::<Option<String>,_>("outcome").ok().flatten(),
                    "at": r.try_get::<chrono::DateTime<chrono::Utc>,_>("occurred_at")
                        .map(|t| t.to_rfc3339()).unwrap_or_default(),
                })
            })
            .collect();
            Ok(json!({
                "created": created,
                "changed": changed,
                "viewed": viewed,
                "log": log,
            }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

// ---------------------------------------------------------------------------
// API tokens (docs/09 §3): bearer credentials bound to a principal — the
// production authentication channel for service accounts and the MCP server.
// Only the sha256 hash is stored; the plaintext is returned exactly once.
// ---------------------------------------------------------------------------

/// Generate a new presented token: `egr_` + 64 lowercase hex chars.
fn generate_api_token() -> String {
    use argon2::password_hash::rand_core::{OsRng, RngCore};
    let mut buf = [0u8; 32];
    OsRng.fill_bytes(&mut buf);
    format!("{}{}", crate::authn::API_TOKEN_PREFIX, hex::encode(buf))
}

#[derive(Debug, Deserialize)]
pub struct TokenInput {
    pub name: String,
    #[serde(default)]
    pub expires_in_days: Option<i64>,
}

/// `POST /users/<id>/tokens` — issue an API token. The plaintext token is in
/// the response body once; afterwards only metadata is listable.
#[post("/users/<principal_id>/tokens", data = "<body>")]
pub async fn create_token(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
    body: Json<TokenInput>,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 128 {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let days = body.expires_in_days;
    if matches!(days, Some(d) if d <= 0 || d > 3650) {
        return Err(ApiError::from(Status::UnprocessableEntity));
    }
    let token = generate_api_token();
    let hash = crate::authn::hash_api_token(&token);
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.token.create").await?;
            let exists: Option<(uuid::Uuid,)> = sqlx::query_as(
                "SELECT principal_id FROM sec.principal
                 WHERE tenant_id=$1 AND principal_id=$2",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_optional(&mut *tx)
            .await
            .map_err(db_err)?;
            if exists.is_none() {
                return Err(http_err(404, "user not found"));
            }
            // Minting an API token for a special-role member hands the
            // caller a superadmin credential — restrict to superadmins.
            if principal_is_special(&mut tx, ctx.tenant_id, pid).await?
                && !caller_is_superadmin(&mut tx, ctx.tenant_id, ctx.principal_id).await?
            {
                return Err(http_err(403, "special-role accounts require superadmin"));
            }
            let token_id = uuid::Uuid::now_v7();
            let row: (
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            ) = sqlx::query_as(
                "INSERT INTO sec.api_token
                     (token_id, tenant_id, principal_id, name, token_hash, created_by,
                      expires_at)
                     VALUES ($1,$2,$3,$4,$5,$6,
                             CASE WHEN $7::bigint IS NULL THEN NULL
                                  ELSE now() + ($7 || ' days')::interval END)
                     RETURNING created_at, expires_at",
            )
            .bind(token_id)
            .bind(ctx.tenant_id)
            .bind(pid)
            .bind(&name)
            .bind(&hash)
            .bind(ctx.principal_id)
            .bind(days)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
            tx.commit().await.map_err(db_err)?;
            Ok(json!({
                "token_id": token_id.to_string(),
                "token": token,
                "name": name,
                "created_at": row.0.to_rfc3339(),
                "expires_at": row.1.map(|t| t.to_rfc3339()),
            }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `GET /users/<id>/tokens` — token metadata (never the secret material).
#[get("/users/<principal_id>/tokens")]
pub async fn list_tokens(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.token.list").await?;
            let exists: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM sec.principal
                               WHERE tenant_id=$1 AND principal_id=$2)",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_one(&mut *tx)
            .await
            .map_err(db_err)?;
            if !exists {
                return Err(http_err(404, "user not found"));
            }
            let rows = sqlx::query(
                "SELECT token_id, name, created_at, expires_at, revoked_at,
                        last_used_at
                 FROM sec.api_token
                 WHERE tenant_id=$1 AND principal_id=$2
                 ORDER BY created_at DESC",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .fetch_all(&mut *tx)
            .await
            .map_err(db_err)?;
            let out: Vec<Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "token_id": r.try_get::<uuid::Uuid,_>("token_id")
                            .unwrap_or_default().to_string(),
                        "name": r.try_get::<String,_>("name").unwrap_or_default(),
                        "created_at": r.try_get::<chrono::DateTime<chrono::Utc>,_>("created_at")
                            .map(|t| t.to_rfc3339()).unwrap_or_default(),
                        "expires_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("expires_at")
                            .ok().flatten().map(|t| t.to_rfc3339()),
                        "revoked_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("revoked_at")
                            .ok().flatten().map(|t| t.to_rfc3339()),
                        "last_used_at": r.try_get::<Option<chrono::DateTime<chrono::Utc>>,_>("last_used_at")
                            .ok().flatten().map(|t| t.to_rfc3339()),
                    })
                })
                .collect();
            Ok(Value::Array(out))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

/// `DELETE /users/<id>/tokens/<token_id>` — revoke a token. Idempotent:
/// revoking an already-revoked token reports `revoked: true` again.
#[delete("/users/<principal_id>/tokens/<token_id>")]
pub async fn revoke_token(
    cap: CapabilityGuard,
    state: &State<AppState>,
    principal_id: String,
    token_id: String,
) -> Result<Json<ApiEnvelope<Value>>, ApiError> {
    let exec = pg(state)?;
    let ctx = cap.ctx.clone();
    let pid: uuid::Uuid = principal_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let tid: uuid::Uuid = token_id.parse().map_err(|_| ApiError::from(Status::NotFound))?;
    let v = exec
        .run(Deadline::in_secs(10), move |pool| async move {
            let mut tx = audit_tx(&pool, &ctx, "users.token.revoke").await?;
            if principal_is_special(&mut tx, ctx.tenant_id, pid).await?
                && !caller_is_superadmin(&mut tx, ctx.tenant_id, ctx.principal_id).await?
            {
                return Err(http_err(403, "special-role accounts require superadmin"));
            }
            let changed = sqlx::query(
                "UPDATE sec.api_token SET revoked_at=now()
                 WHERE tenant_id=$1 AND principal_id=$2 AND token_id=$3
                   AND revoked_at IS NULL",
            )
            .bind(ctx.tenant_id)
            .bind(pid)
            .bind(tid)
            .execute(&mut *tx)
            .await
            .map_err(db_err)?
            .rows_affected();
            if changed == 0 {
                let exists: Option<(uuid::Uuid,)> = sqlx::query_as(
                    "SELECT token_id FROM sec.api_token
                     WHERE tenant_id=$1 AND principal_id=$2 AND token_id=$3",
                )
                .bind(ctx.tenant_id)
                .bind(pid)
                .bind(tid)
                .fetch_optional(&mut *tx)
                .await
                .map_err(db_err)?;
                if exists.is_none() {
                    return Err(http_err(404, "token not found"));
                }
            }
            tx.commit().await.map_err(db_err)?;
            Ok(json!({ "token_id": tid.to_string(), "revoked": true }))
        })
        .await
        .map_err(|e| run_err(&e))?;
    Ok(Json(ApiEnvelope {
        data: v,
        meta: meta(),
    }))
}

pub fn routes() -> Vec<rocket::Route> {
    rocket::routes![
        list_role_groups,
        list_users,
        create_user,
        get_user,
        update_user,
        delete_user,
        user_activity,
        create_token,
        list_tokens,
        revoke_token,
    ]
}
