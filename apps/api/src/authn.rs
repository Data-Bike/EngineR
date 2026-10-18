//! Session authentication + authorization plumbing (docs/08 §2-4, docs/09).
//!
//! All database access goes through the `AsyncStdDbExecutor` bridge: handlers
//! queue a closure that runs on the async-std runtime with a `PgPool` and
//! returns an owned `serde_json::Value`.
//!
//! Session model (production):
//! - `sec.principal_credential` holds the globally-unique login + argon2id PHC
//!   hash; it is deliberately outside RLS because the tenant is not yet known
//!   at login time.
//! - `sec.session` is outside RLS for the same reason — the session cookie IS
//!   the credential presented; tenant scoping is enforced in code.
//! - Every tenant-scoped query runs on a single pooled connection with the RLS
//!   context installed session-locally and reset before release.

use std::sync::Arc;

use argon2::password_hash::{PasswordHash, PasswordVerifier};
use argon2::Argon2;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use enginer_persistence::RlsContext;
use enginer_security_api::{local_client, ActionDecisionRequest, SecurityContextDto};
use serde::{Deserialize, Serialize};

/// Resolved, authenticated request context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    pub session_id: uuid::Uuid,
    pub tenant_id: uuid::Uuid,
    pub principal_id: uuid::Uuid,
    pub display_name: String,
    pub auth_strength: String,
    pub groups_revision: i64,
    pub csrf_token: String,
    /// `true` when the request was authenticated by an `Authorization:
    /// Bearer` API token rather than the session cookie. Bearer auth is not
    /// cookie-bound, so CSRF echo does not apply to it.
    #[serde(default)]
    pub via_bearer: bool,
}

/// Authenticated principal profile for `/me/context`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrincipalProfile {
    pub ctx: AuthContext,
    pub capabilities: Vec<String>,
    pub policy_revision: i64,
    /// UI URL rules: `{deny: [...], allowlist: [...]|null}`.
    pub ui_rules: serde_json::Value,
}

fn to_db_error(e: sqlx::Error) -> DbError {
    DbError::Other {
        message: e.to_string(),
    }
}

fn err_json(e: &impl std::fmt::Display) -> serde_json::Value {
    serde_json::json!({ "error": e.to_string() })
}

fn rls(tenant: uuid::Uuid, principal: uuid::Uuid, purpose: &str) -> RlsContext {
    RlsContext {
        tenant_id: enginer_domain::TenantId(tenant),
        principal_id: enginer_domain::PrincipalId(principal),
        purpose: purpose.to_string(),
        policy_revision: None,
    }
}

/// Begin a transaction with the RLS context installed transaction-locally.
/// The context is cleared automatically on commit/rollback — nothing leaks
/// into pooled connections.
async fn begin_rls<'e>(
    pool: &'e sqlx::postgres::PgPool,
    ctx: &RlsContext,
) -> Result<sqlx::Transaction<'e, sqlx::Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    ctx.install_tx(&mut tx)
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?;
    Ok(tx)
}

/// Look up a credential row by globally-unique login. Runs without tenant
/// context (the credential table is a pre-auth directory).
pub async fn find_credential(
    exec: &Arc<AsyncStdDbExecutor>,
    login: &str,
) -> Result<Option<serde_json::Value>, DbError> {
    let login = login.to_string();
    exec.run(Deadline::in_secs(5), move |pool| async move {
        let row: Option<(
            uuid::Uuid,
            uuid::Uuid,
            String,
            String,
            Option<chrono::DateTime<chrono::Utc>>,
        )> = sqlx::query_as(
            "SELECT tenant_id, principal_id, password_hash, status, locked_until
             FROM sec.principal_credential
             WHERE login = $1",
        )
        .bind(&login)
        .fetch_optional(&pool)
        .await
        .map_err(to_db_error)?;
        match row {
            None => Ok(serde_json::Value::Null),
            Some((tenant, principal, hash, status, locked)) => Ok(serde_json::json!({
                "tenant_id": tenant,
                "principal_id": principal,
                "password_hash": hash,
                "status": status,
                "locked_until": locked,
            })),
        }
    })
    .await
    .map(Some)
}

/// Verify an argon2id PHC string against a plaintext password.
pub fn verify_password(phc: &str, password: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

/// Load principal row (active check + display name) under tenant context.
pub async fn load_principal(
    exec: &Arc<AsyncStdDbExecutor>,
    tenant: uuid::Uuid,
    principal: uuid::Uuid,
) -> Result<Option<(String, String)>, DbError> {
    exec.run(Deadline::in_secs(5), move |pool| async move {
        let mut tx = begin_rls(&pool, &rls(tenant, principal, "authn"))
            .await
            .map_err(to_db_error)?;
        let row: Option<(String, String)> = sqlx::query_as(
            "SELECT display_name, status FROM sec.principal
             WHERE tenant_id = $1 AND principal_id = $2",
        )
        .bind(tenant)
        .bind(principal)
        .fetch_optional(&mut *tx)
        .await
        .map_err(to_db_error)?;
        tx.commit().await.map_err(to_db_error)?;
        Ok(match row {
            None => serde_json::Value::Null,
            Some((name, status)) => {
                serde_json::json!({ "display_name": name, "status": status })
            }
        })
    })
    .await
    .map(|v| {
        if v.is_null() {
            None
        } else {
            Some((
                v["display_name"].as_str().unwrap_or_default().to_string(),
                v["status"].as_str().unwrap_or_default().to_string(),
            ))
        }
    })
}

/// Insert a session row and refresh `principal.last_seen_at`.
pub async fn create_session(
    exec: &Arc<AsyncStdDbExecutor>,
    tenant: uuid::Uuid,
    principal: uuid::Uuid,
    auth_strength: &str,
    csrf_token: &str,
    user_agent: Option<&str>,
    ip: Option<String>,
    ttl_hours: i64,
) -> Result<uuid::Uuid, DbError> {
    let auth_strength = auth_strength.to_string();
    let csrf = csrf_token.to_string();
    let ua = user_agent.map(str::to_string);
    let v = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let session_id = uuid::Uuid::now_v7();
            // sec.session is outside RLS (pre-context credential lookup), but
            // the principal touch needs the tenant context — one transaction
            // covers both.
            let mut tx = begin_rls(&pool, &rls(tenant, principal, "authn"))
                .await
                .map_err(to_db_error)?;
            sqlx::query(
                "INSERT INTO sec.session
                 (session_id, tenant_id, principal_id, auth_strength, expires_at,
                  groups_revision, csrf_token, user_agent, ip_address)
                 VALUES ($1, $2, $3, $4, now() + ($5 || ' hours')::interval, 1, $6, $7, $8)",
            )
            .bind(session_id)
            .bind(tenant)
            .bind(principal)
            .bind(&auth_strength)
            .bind(ttl_hours)
            .bind(&csrf)
            .bind(&ua)
            .bind(ip)
            .execute(&mut *tx)
            .await
            .map_err(to_db_error)?;
            sqlx::query(
                "UPDATE sec.principal SET last_seen_at = now()
                 WHERE tenant_id = $1 AND principal_id = $2",
            )
            .bind(tenant)
            .bind(principal)
            .execute(&mut *tx)
            .await
            .map_err(to_db_error)?;
            tx.commit().await.map_err(to_db_error)?;
            Ok(serde_json::json!({ "session_id": session_id }))
        })
        .await?;
    Ok(
        uuid::Uuid::parse_str(v["session_id"].as_str().unwrap_or_default())
            .unwrap_or_else(|_| uuid::Uuid::nil()),
    )
}

/// Resolve a session cookie into an `AuthContext`. Returns `None` for
/// missing/expired/ended sessions or inactive principals (fail closed).
pub async fn resolve_session(
    exec: &Arc<AsyncStdDbExecutor>,
    session_id: uuid::Uuid,
) -> Result<Option<AuthContext>, DbError> {
    let v = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let row: Option<(
                uuid::Uuid,
                uuid::Uuid,
                String,
                Option<String>,
                chrono::DateTime<chrono::Utc>,
            )> = sqlx::query_as(
                "SELECT tenant_id, principal_id, auth_strength, csrf_token, expires_at
                 FROM sec.session
                 WHERE session_id = $1 AND ended_at IS NULL AND expires_at > now()",
            )
            .bind(session_id)
            .fetch_optional(&pool)
            .await
            .map_err(to_db_error)?;
            let Some((tenant, principal, strength, csrf, _exp)) = row else {
                return Ok(serde_json::Value::Null);
            };
            // Principal must still be active; read under its tenant context.
            let mut tx = begin_rls(&pool, &rls(tenant, principal, "authn"))
                .await
                .map_err(to_db_error)?;
            let prow: Option<(String, String)> = sqlx::query_as(
                "SELECT display_name, status FROM sec.principal
                 WHERE tenant_id = $1 AND principal_id = $2",
            )
            .bind(tenant)
            .bind(principal)
            .fetch_optional(&mut *tx)
            .await
            .map_err(to_db_error)?;
            tx.commit().await.map_err(to_db_error)?;
            let Some((display_name, status)) = prow else {
                return Ok(serde_json::Value::Null);
            };
            if status != "active" {
                return Ok(serde_json::Value::Null);
            }
            Ok(serde_json::json!({
                "session_id": session_id,
                "tenant_id": tenant,
                "principal_id": principal,
                "display_name": display_name,
                "auth_strength": strength,
                "csrf_token": csrf.unwrap_or_default(),
            }))
        })
        .await?;
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(AuthContext {
        session_id: v["session_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        tenant_id: v["tenant_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        principal_id: v["principal_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        display_name: v["display_name"].as_str().unwrap_or_default().to_string(),
        auth_strength: v["auth_strength"].as_str().unwrap_or_default().to_string(),
        groups_revision: 0,
        csrf_token: v["csrf_token"].as_str().unwrap_or_default().to_string(),
        via_bearer: false,
    }))
}

/// Presented API token prefix — `egr_` followed by 64 lowercase hex chars.
pub const API_TOKEN_PREFIX: &str = "egr_";

/// Hash a presented API token for lookup (sha256 hex).
pub fn hash_api_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(token.as_bytes());
    hex::encode(h.finalize())
}

/// Resolve an `Authorization: Bearer egr_…` token into an `AuthContext`.
///
/// The token table is outside RLS (the token is the credential), the lookup
/// binds the sha256 hash; the principal is then re-checked for `active`
/// under its tenant context. `session_id` in the context carries the
/// `token_id` so audit attribution still works. Returns `None` for
/// malformed/expired/revoked tokens or inactive principals (fail closed).
pub async fn resolve_bearer(
    exec: &Arc<AsyncStdDbExecutor>,
    token: &str,
) -> Result<Option<AuthContext>, DbError> {
    if !token.starts_with(API_TOKEN_PREFIX) || token.len() != 4 + 64 {
        return Ok(None);
    }
    let hash = hash_api_token(token);
    let v = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let row: Option<(uuid::Uuid, uuid::Uuid, uuid::Uuid)> = sqlx::query_as(
                "SELECT token_id, tenant_id, principal_id FROM sec.api_token
                 WHERE token_hash = $1 AND revoked_at IS NULL
                   AND (expires_at IS NULL OR expires_at > now())",
            )
            .bind(&hash)
            .fetch_optional(&pool)
            .await
            .map_err(to_db_error)?;
            let Some((token_id, tenant, principal)) = row else {
                return Ok(serde_json::Value::Null);
            };
            let mut tx = begin_rls(&pool, &rls(tenant, principal, "authn.bearer"))
                .await
                .map_err(to_db_error)?;
            let prow: Option<(String, String)> = sqlx::query_as(
                "SELECT display_name, status FROM sec.principal
                 WHERE tenant_id = $1 AND principal_id = $2",
            )
            .bind(tenant)
            .bind(principal)
            .fetch_optional(&mut *tx)
            .await
            .map_err(to_db_error)?;
            let Some((display_name, status)) = prow else {
                return Ok(serde_json::Value::Null);
            };
            if status != "active" {
                return Ok(serde_json::Value::Null);
            }
            tx.commit().await.map_err(to_db_error)?;
            // Best-effort usage bookkeeping, outside the read transaction.
            let _ =
                sqlx::query("UPDATE sec.api_token SET last_used_at = now() WHERE token_id = $1")
                    .bind(token_id)
                    .execute(&pool)
                    .await;
            Ok(serde_json::json!({
                "session_id": token_id,
                "tenant_id": tenant,
                "principal_id": principal,
                "display_name": display_name,
            }))
        })
        .await?;
    if v.is_null() {
        return Ok(None);
    }
    Ok(Some(AuthContext {
        session_id: v["session_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        tenant_id: v["tenant_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        principal_id: v["principal_id"]
            .as_str()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default(),
        display_name: v["display_name"].as_str().unwrap_or_default().to_string(),
        auth_strength: "api_token".to_string(),
        groups_revision: 0,
        csrf_token: String::new(),
        via_bearer: true,
    }))
}

/// Mark a session ended (logout).
pub async fn end_session(
    exec: &Arc<AsyncStdDbExecutor>,
    session_id: uuid::Uuid,
) -> Result<(), DbError> {
    exec.run(Deadline::in_secs(5), move |pool| async move {
        sqlx::query("UPDATE sec.session SET ended_at = now() WHERE session_id = $1")
            .bind(session_id)
            .execute(&pool)
            .await
            .map_err(to_db_error)?;
        Ok(serde_json::json!({ "ok": true }))
    })
    .await
    .map(|_| ())
}

/// Compute the effective capability set for `/me/context`:
/// actions allowed by the published policy revision through the principal's
/// capability-group memberships, minus explicitly denied ones.
pub async fn principal_profile(
    exec: &Arc<AsyncStdDbExecutor>,
    ctx: &AuthContext,
) -> Result<Option<PrincipalProfile>, DbError> {
    let ctx = ctx.clone();
    let v = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let tenant = ctx.tenant_id;
            let principal = ctx.principal_id;
            let mut tx = begin_rls(&pool, &rls(tenant, principal, "me.context"))
                .await
                .map_err(to_db_error)?;
            let rev: Option<(i64,)> = sqlx::query_as(
                "SELECT revision_id FROM sec.policy_revision
                 WHERE tenant_id = $1 AND state = 'published'
                 ORDER BY published_at DESC NULLS LAST LIMIT 1",
            )
            .bind(tenant)
            .fetch_optional(&mut *tx)
            .await
            .map_err(to_db_error)?;
            let rev = rev.map(|(r,)| r).unwrap_or(0);

            // Capability rules of all effective groups — direct memberships
            // plus ancestors through parent_id (role inheritance copies the
            // parent's rights). App-clock tolerance on valid_from.
            let rows: Vec<(String, String)> = sqlx::query_as(
                "WITH RECURSIVE eff AS (
                     SELECT g.group_id, g.parent_id
                     FROM sec.principal_group_membership m
                     JOIN sec.\"group\" g ON g.group_id = m.group_id
                     WHERE m.tenant_id = $1 AND m.principal_id = $3
                       AND (m.valid_to IS NULL OR m.valid_to >= now())
                       AND (m.valid_from <= now() + interval '120 seconds')
                     UNION
                     SELECT p.group_id, p.parent_id
                     FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
                     WHERE p.tenant_id = $1
                 )
                 SELECT DISTINCT pr.action, pr.effect
                 FROM sec.policy_rule pr
                 JOIN sec.\"group\" g
                   ON g.group_id = pr.group_id AND g.tenant_id = pr.tenant_id
                 WHERE pr.tenant_id = $1 AND pr.policy_revision = $2
                   AND g.class = 'capability'
                   AND pr.group_id IN (SELECT group_id FROM eff)
                   AND (pr.valid_to IS NULL OR pr.valid_to >= now())
                   AND pr.valid_from <= now() + interval '120 seconds'",
            )
            .bind(tenant)
            .bind(rev)
            .bind(principal)
            .fetch_all(&mut *tx)
            .await
            .map_err(to_db_error)?;

            let grev: (Option<i64>,) = sqlx::query_as(
                "SELECT max(groups_revision) FROM sec.principal_group_membership
                 WHERE tenant_id = $1 AND principal_id = $2",
            )
            .bind(tenant)
            .bind(principal)
            .fetch_one(&mut *tx)
            .await
            .map_err(to_db_error)?;

            // Special roles resolved through the role tree (parent_id chain).
            let flags: Vec<String> = sqlx::query_scalar(
                "WITH RECURSIVE eff AS (
                     SELECT g.group_id, g.parent_id
                     FROM sec.principal_group_membership m
                     JOIN sec.\"group\" g ON g.group_id = m.group_id
                     WHERE m.tenant_id = $1 AND m.principal_id = $2
                       AND (m.valid_to IS NULL OR m.valid_to >= now())
                       AND (m.valid_from <= now() + interval '120 seconds')
                     UNION
                     SELECT p.group_id, p.parent_id
                     FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
                     WHERE p.tenant_id = $1
                 )
                 SELECT DISTINCT COALESCE(NULLIF(g.special,''), g.alias)
                 FROM sec.\"group\" g
                 WHERE g.group_id IN (SELECT group_id FROM eff)
                   AND COALESCE(NULLIF(g.special,''), g.alias)
                       IN ('superadmin','superuser')",
            )
            .bind(tenant)
            .bind(principal)
            .fetch_all(&mut *tx)
            .await
            .map_err(to_db_error)?;

            // UI URL rules (system.url with a non-/api path) for menu/route
            // hiding on the client; API paths are enforced server-side.
            let url_rows: Vec<(String, Option<serde_json::Value>)> = sqlx::query_as(
                "WITH RECURSIVE eff AS (
                     SELECT g.group_id, g.parent_id
                     FROM sec.principal_group_membership m
                     JOIN sec.\"group\" g ON g.group_id = m.group_id
                     WHERE m.tenant_id = $1 AND m.principal_id = $2
                       AND (m.valid_to IS NULL OR m.valid_to >= now())
                       AND (m.valid_from <= now() + interval '120 seconds')
                     UNION
                     SELECT p.group_id, p.parent_id
                     FROM sec.\"group\" p JOIN eff e ON e.parent_id = p.group_id
                     WHERE p.tenant_id = $1
                 )
                 SELECT DISTINCT r.effect, r.resource
                 FROM sec.policy_rule r
                 WHERE r.tenant_id = $1 AND r.action = 'system.url'
                   AND r.group_id IN (SELECT group_id FROM eff)
                   AND r.valid_from <= now() + interval '120 seconds'
                   AND (r.valid_to IS NULL OR r.valid_to >= now())",
            )
            .bind(tenant)
            .bind(principal)
            .fetch_all(&mut *tx)
            .await
            .map_err(to_db_error)?;
            tx.commit().await.map_err(to_db_error)?;

            let out = (rev, rows, grev.0.unwrap_or(1), flags, url_rows);

            let (rev, rows, grev, flags, url_rows) = out;
            let denied: std::collections::HashSet<String> = rows
                .iter()
                .filter(|(_, e)| e == "deny")
                .map(|(a, _)| a.clone())
                .collect();
            let mut caps: Vec<String> = rows
                .into_iter()
                .filter(|(a, e)| e == "allow" && !denied.contains(a))
                .map(|(a, _)| a)
                .collect();
            if flags.iter().any(|f| f == "superadmin") {
                caps = enginer_security_domain::CapabilityAction::all()
                    .iter()
                    .map(|a| a.as_str().to_string())
                    .collect();
            } else if flags.iter().any(|f| f == "superuser") {
                caps = enginer_security_domain::CapabilityAction::all()
                    .iter()
                    .map(|a| a.as_str().to_string())
                    .filter(|a| !crate::dataperm::ADMIN_CAPS.contains(&a.as_str()))
                    .collect();
            }
            let caps = caps;
            // UI URL rules mirror dataperm::can_url exactly: rules carry
            // (path prefix, allow) and are evaluated longest-prefix-first;
            // `blanket` means an unmatched path is denied (allowlist mode).
            let mut rules: Vec<serde_json::Value> = Vec::new();
            let mut blanket = false;
            for (effect, resource) in &url_rows {
                let path = resource
                    .as_ref()
                    .and_then(|r| r.get("path"))
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("/");
                if path.starts_with("/api") {
                    continue;
                }
                if (path == "/" || path == "*") && effect == "deny" {
                    blanket = true;
                } else {
                    rules.push(serde_json::json!({
                        "path": path, "allow": effect != "deny",
                    }));
                }
            }
            rules.sort_by(|a, b| {
                let la = a["path"].as_str().map(str::len).unwrap_or(0);
                let lb = b["path"].as_str().map(str::len).unwrap_or(0);
                lb.cmp(&la)
            });
            let ui_rules = if flags.iter().any(|f| f == "superadmin") {
                serde_json::json!({"rules": [], "blanket": false})
            } else if flags.iter().any(|f| f == "superuser") {
                serde_json::json!({
                    "rules": [{"path": "/admin", "allow": false}],
                    "blanket": false,
                })
            } else {
                serde_json::json!({"rules": rules, "blanket": blanket})
            };
            Ok(serde_json::json!({
                "capabilities": caps,
                "policy_revision": rev,
                "groups_revision": grev,
                "ui_rules": ui_rules,
            }))
        })
        .await?;
    if v.is_null() {
        return Ok(None);
    }
    let mut ctx = ctx;
    ctx.groups_revision = v["groups_revision"].as_i64().unwrap_or(1);
    Ok(Some(PrincipalProfile {
        ctx,
        capabilities: v["capabilities"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
        policy_revision: v["policy_revision"].as_i64().unwrap_or(0),
        ui_rules: v["ui_rules"].clone(),
    }))
}

/// Evaluate an action decision through the PDP with the real snapshot provider
/// (deny-overrides). Returns `true` on Allow / AllowWithObligations.
pub async fn decide_action(
    exec: &Arc<AsyncStdDbExecutor>,
    ctx: &AuthContext,
    action: &str,
    resource_kind: &str,
    resource_id: Option<uuid::Uuid>,
) -> Result<bool, DbError> {
    let action = action.to_string();
    let resource_kind = resource_kind.to_string();
    let ctx = ctx.clone();
    let v = exec
        .run(Deadline::in_secs(5), move |pool| async move {
            let provider = enginer_persistence::PgSecuritySnapshotProvider::new(Arc::new(pool));
            let client = local_client(Arc::new(provider));
            let req = ActionDecisionRequest {
                context: SecurityContextDto {
                    request_id: uuid::Uuid::now_v7().to_string(),
                    tenant_id: ctx.tenant_id,
                    principal_id: ctx.principal_id,
                    session_id: ctx.session_id,
                    auth_strength: ctx.auth_strength.clone(),
                    groups_revision: ctx.groups_revision,
                    purpose: "http".to_string(),
                    locale: "ru".to_string(),
                    time: chrono::Utc::now(),
                    client_service: "enginer-api".to_string(),
                },
                action,
                resource_kind,
                resource_id,
                parameters: serde_json::Map::new(),
            };
            match client.action_decision(req) {
                Ok(resp) => Ok(serde_json::json!({
                    "effect": resp.effect,
                    "policy_revision": resp.policy_revision,
                })),
                Err(e) => Ok(err_json(&e)),
            }
        })
        .await?;
    if let Some(e) = v.get("error") {
        return Err(DbError::Other {
            message: e.as_str().unwrap_or("pdp error").to_string(),
        });
    }
    let effect = v["effect"].as_str().unwrap_or("deny");
    Ok(effect == "allow" || effect == "allow_with_obligations")
}
