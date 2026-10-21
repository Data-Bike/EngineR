//! Authentication routes backed by `sec.principal_credential` + `sec.session`
//! (docs/08 §2-3, docs/16 §5).
//!
//! - `POST /api/v1/auth/login` — argon2id verify, create session row, set the
//!   `session` cookie (HttpOnly, SameSite=Lax) and return the CSRF token.
//! - `GET  /api/v1/me/context` — session resolution + effective capabilities
//!   computed from the published policy revision.
//! - `POST /api/v1/auth/logout` — end the session row, clear the cookie.
//! - `POST /api/v1/auth/refresh` — extend the session expiry if still valid.

use rocket::http::{Cookie, CookieJar, SameSite, Status};
use rocket::request::{self, FromRequest, Request};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post, routes};
use serde::{Deserialize, Serialize};

use crate::authn;
use crate::AppState;
use crate::ApiError;

/// The `x-csrf-token` header value (empty string when absent) — used by
/// cookie-authenticated mutations that don't go through `CapabilityGuard`.
pub struct CsrfHeader(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CsrfHeader {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(CsrfHeader(
            req.headers()
                .get_one("x-csrf-token")
                .unwrap_or_default()
                .to_string(),
        ))
    }
}

/// Client metadata captured for session audit (user-agent + peer IP).
pub struct ClientMeta {
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientMeta {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(ClientMeta {
            user_agent: req.headers().get_one("user-agent").map(str::to_string),
            ip: req.client_ip().map(|a| a.to_string()),
        })
    }
}

/// Optional `Authorization: Bearer egr_…` API token extracted from the
/// request. `None` when the header is absent or not an API token.
pub struct BearerToken<'a>(pub Option<&'a str>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for BearerToken<'r> {
    type Error = std::convert::Infallible;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let token = req
            .headers()
            .get_one("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| t.starts_with(crate::authn::API_TOKEN_PREFIX));
        request::Outcome::Success(BearerToken(token))
    }
}

/// Login request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

/// Login response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub principal_id: String,
    pub display_name: String,
    pub tenant_id: String,
    pub csrf_token: String,
}

/// User context (docs/16 §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub principal_id: String,
    pub display_name: String,
    pub tenant_id: String,
    pub locale: String,
    pub timezone: String,
    pub auth_strength: String,
    pub groups_revision: i64,
    pub capabilities: Vec<String>,
    pub workspaces: Vec<String>,
    pub policy_revision: i64,
    pub csrf_token: String,
    /// Frontend route/menu access rules derived from the principal's
    /// system.url policy rules (`{deny:[..], allowlist:[..]|null}`).
    pub ui_rules: serde_json::Value,
    /// Catalogs the principal can read (docs/28 §6): drives the
    /// catalog-selection menu in the UI. Empty when no catalog rule grants
    /// access — the UI then shows nothing catalog-scoped at all.
    pub catalogs: Vec<serde_json::Value>,
}

/// API envelope wrapper.
#[derive(Debug, Clone, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: serde_json::Value,
}

fn meta() -> serde_json::Value {
    serde_json::json!({ "request_id": uuid::Uuid::now_v7().to_string() })
}

fn session_cookie(session_id: uuid::Uuid, ttl_hours: i64) -> Cookie<'static> {
    let mut c = Cookie::new("session", session_id.to_string());
    c.set_http_only(true);
    c.set_same_site(SameSite::Lax);
    c.set_path("/");
    c.set_max_age(rocket::time::Duration::hours(ttl_hours));
    // `Secure` is on by default — behind a TLS-terminating proxy Rocket's
    // auto-detection cannot see TLS, so relying on it leaves the cookie
    // unprotected. `ENGINER_INSECURE_COOKIES=1` opts out for plain-HTTP
    // dev deployments (localhost counts as a secure context in browsers,
    // so local dev keeps working with the flag unset too).
    c.set_secure(std::env::var_os("ENGINER_INSECURE_COOKIES").is_none());
    c
}

const SESSION_TTL_HOURS: i64 = 12;

/// Sliding-window login rate limit per client IP. In-memory and
/// best-effort — a multi-instance deployment should add an edge limiter,
/// but a single API instance still must not let one source hammer
/// `/auth/login` unboundedly.
fn login_rate_allow(ip: &str) -> bool {
    const WINDOW: std::time::Duration = std::time::Duration::from_secs(300);
    const MAX_ATTEMPTS: usize = 20;
    const MAX_TRACKED: usize = 10_000;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static BUCKET: OnceLock<Mutex<HashMap<String, std::collections::VecDeque<std::time::Instant>>>> =
        OnceLock::new();
    let map = BUCKET.get_or_init(|| Mutex::new(HashMap::new()));
    let now = std::time::Instant::now();
    let Ok(mut map) = map.lock() else {
        return true; // poisoned lock must not take login down
    };
    if map.len() >= MAX_TRACKED {
        map.clear();
    }
    let attempts = map.entry(ip.to_string()).or_default();
    while attempts.front().is_some_and(|t| now.duration_since(*t) > WINDOW) {
        attempts.pop_front();
    }
    if attempts.len() >= MAX_ATTEMPTS {
        return false;
    }
    attempts.push_back(now);
    true
}

#[post("/auth/login", data = "<body>")]
pub async fn login(
    state: &State<AppState>,
    body: Json<LoginBody>,
    cookies: &CookieJar<'_>,
    client: ClientMeta,
) -> Result<Json<ApiEnvelope<LoginResponse>>, ApiError> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(ApiError::from(Status::ServiceUnavailable));
    };
    let body = body.into_inner();
    let login_name = body.username.trim().to_lowercase();
    let client_ip = client.ip.clone();
    if login_name.is_empty() || body.password.is_empty() {
        return Err(ApiError::from(Status::Unauthorized));
    }
    // Per-IP attempt budget (docs/16 §13): the per-login lockout stops
    // password guessing on one account, but a single source spraying many
    // logins could also re-trigger every lockout window (lockout DoS).
    // Counted before any credential work so success/failure timing stays
    // uniform.
    if !login_rate_allow(client_ip.as_deref().unwrap_or("unknown")) {
        return Err(ApiError::from(Status::TooManyRequests));
    }

    let cred = authn::find_credential(&exec, &login_name)
        .await
        .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
        .unwrap_or(serde_json::Value::Null);

    // Uniform failure: no oracle on whether the login exists, is locked, or
    // the password is wrong.
    let audit_failure = |cred: &serde_json::Value| {
        rocket::tokio::spawn(crate::audit_pg::record_audit_event(
            exec.clone(),
            cred["tenant_id"]
                .as_str()
                .and_then(|s| s.parse::<uuid::Uuid>().ok())
                .unwrap_or(uuid::Uuid::nil()),
            cred["principal_id"]
                .as_str()
                .and_then(|s| s.parse::<uuid::Uuid>().ok()),
            None,
            "authentication",
            "auth.login",
            "session",
            None,
            Some("deny"),
            "denied",
            client_ip.clone(),
            serde_json::json!({ "login": login_name }),
        ));
    };
    let unauthorized = || {
        audit_failure(&cred);
        Err(ApiError::from(Status::Unauthorized))
    };
    if cred.is_null() {
        // Burn an argon2 verify anyway so a missing login takes the same
        // time as a wrong password — no timing oracle on existence.
        let _ = authn::verify_password(authn::DUMMY_PHC, &body.password);
        return unauthorized();
    }
    if cred["status"].as_str() != Some("active") {
        let _ = authn::verify_password(authn::DUMMY_PHC, &body.password);
        return unauthorized();
    }
    if let Some(locked) = cred["locked_until"].as_str() {
        if chrono::DateTime::parse_from_rfc3339(locked)
            .map(|t| t > chrono::Utc::now())
            .unwrap_or(false)
        {
            let _ = authn::verify_password(authn::DUMMY_PHC, &body.password);
            return unauthorized();
        }
    }
    let hash = cred["password_hash"].as_str().unwrap_or_default();
    if !authn::verify_password(hash, &body.password) {
        authn::record_failed_login(&exec, &login_name).await;
        return unauthorized();
    }
    authn::reset_failed_logins(&exec, &login_name).await;

    let tenant: uuid::Uuid = cred["tenant_id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::from(Status::InternalServerError))?;
    let principal: uuid::Uuid = cred["principal_id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiError::from(Status::InternalServerError))?;

    let principal_row = authn::load_principal(&exec, tenant, principal)
        .await
        .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
        .ok_or_else(|| ApiError::from(Status::Unauthorized))?;
    if principal_row.1 != "active" {
        return Err(ApiError::from(Status::Unauthorized));
    }

    // login.use_ui gates interactive UI sessions: a capability group may
    // withhold UI login (e.g. API-only service accounts) while Bearer
    // access stays open. Evaluated through the same capability resolution
    // /me/context uses — superadmin bypass and superuser filtering included.
    let gate_ctx = authn::AuthContext {
        session_id: uuid::Uuid::nil(),
        tenant_id: tenant,
        principal_id: principal,
        display_name: principal_row.0.clone(),
        auth_strength: "password".to_string(),
        groups_revision: 0,
        csrf_token: String::new(),
        via_bearer: false,
    };
    let ui_caps = authn::principal_profile(&exec, &gate_ctx)
        .await
        .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
        .map(|p| p.capabilities)
        .unwrap_or_default();
    if !ui_caps.iter().any(|c| c == "login.use_ui") {
        audit_failure(&cred);
        return Err(ApiError::from(Status::Forbidden));
    }

    let csrf = uuid::Uuid::now_v7().to_string();
    let session_id = authn::create_session(
        &exec,
        tenant,
        principal,
        "password",
        &csrf,
        client.user_agent.as_deref(),
        client_ip.clone(),
        SESSION_TTL_HOURS,
    )
    .await
    .map_err(|_| ApiError::from(Status::ServiceUnavailable))?;

    cookies.add(session_cookie(session_id, SESSION_TTL_HOURS));
    rocket::tokio::spawn(crate::audit_pg::record_audit_event(
        exec.clone(),
        tenant,
        Some(principal),
        Some(session_id),
        "authentication",
        "auth.login",
        "session",
        None,
        Some("allow"),
        "success",
        client_ip.clone(),
        serde_json::json!({ "login": login_name }),
    ));
    Ok(Json(ApiEnvelope {
        data: LoginResponse {
            principal_id: principal.to_string(),
            display_name: principal_row.0,
            tenant_id: tenant.to_string(),
            csrf_token: csrf,
        },
        meta: meta(),
    }))
}

#[post("/auth/logout")]
pub async fn logout(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    csrf_header: CsrfHeader,
) -> Result<Json<serde_json::Value>, ApiError> {
    if let (Some(exec), Some(cookie)) = (state.db_executor.clone(), cookies.get("session")) {
        if let Ok(sid) = cookie.value().parse::<uuid::Uuid>() {
            // Cookie-authenticated mutations require the CSRF token the SPA
            // received at login (same contract as /auth/refresh). A request
            // without it did not come from the app — refuse rather than end
            // someone else's session on a cross-site post.
            let session_csrf = exec
                .run(enginer_db_runtime::Deadline::in_secs(5), move |pool| async move {
                    sqlx::query_as::<_, (Option<String>,)>(
                        "SELECT csrf_token FROM sec.session WHERE session_id=$1",
                    )
                    .bind(sid)
                    .fetch_optional(&pool)
                    .await
                    .map(|row| serde_json::json!(row.map(|(c,)| c)))
                    .map_err(|e| enginer_db_runtime::DbError::Other {
                        message: e.to_string(),
                    })
                })
                .await
                .map_err(|_| ApiError::from(Status::ServiceUnavailable))?;
            // `null` = no session row or row without a token — nothing to
            // compare; a stored token must match the header.
            if let Some(stored) = session_csrf.as_str() {
                if stored != csrf_header.0 {
                    return Err(ApiError::from(Status::Forbidden));
                }
            }
            let _ = authn::end_session(&exec, sid).await;
        }
    }
    cookies.remove(Cookie::from("session"));
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[get("/me/context")]
pub async fn me_context(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    bearer: BearerToken<'_>,
) -> Result<Json<ApiEnvelope<UserContext>>, ApiError> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(ApiError::from(Status::ServiceUnavailable));
    };
    // Same two credential channels as the PEP guard: Bearer API token wins
    // over the session cookie when both are present.
    let ctx = if let Some(token) = bearer.0 {
        authn::resolve_bearer(&exec, token)
            .await
            .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
            .ok_or_else(|| ApiError::from(Status::Unauthorized))?
    } else {
        let Some(cookie) = cookies.get("session") else {
            return Err(ApiError::from(Status::Unauthorized));
        };
        let Ok(sid) = cookie.value().parse::<uuid::Uuid>() else {
            return Err(ApiError::from(Status::Unauthorized));
        };
        authn::resolve_session(&exec, sid)
            .await
            .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
            .ok_or_else(|| ApiError::from(Status::Unauthorized))?
    };
    let profile = authn::principal_profile(&exec, &ctx)
        .await
        .map_err(|_| ApiError::from(Status::ServiceUnavailable))?
        .ok_or_else(|| ApiError::from(Status::Unauthorized))?;

    // Catalogs visible to this principal (docs/28 §6): every active
    // catalog row filtered through DataPerm::can_catalog — the same gate
    // the entity routes apply.
    let catalogs = exec
        .run(enginer_db_runtime::Deadline::in_secs(5), {
            let ctx = ctx.clone();
            move |pool| async move {
                let mut tx = crate::entity_pg::rls_tx(&pool, &ctx, "me.catalogs").await?;
                let perm = crate::entity_pg::perm_for(&mut tx, &ctx).await?;
                let rows: Vec<(uuid::Uuid, String, String)> = sqlx::query_as(
                    "SELECT public_id, alias, name FROM core.catalog
                         WHERE tenant_id=$1 AND status='active' ORDER BY alias",
                )
                .bind(ctx.tenant_id)
                .fetch_all(&mut *tx)
                .await
                .map_err(crate::entity_pg::db_err)?;
                let out: Vec<serde_json::Value> = rows
                    .iter()
                    .filter(|(pid, alias, _)| {
                        perm.can_catalog(
                            &[&pid.to_string(), alias.as_str()],
                            crate::dataperm::Op::Read,
                        )
                    })
                    .map(|(pid, alias, name)| {
                        serde_json::json!({
                            "public_id": pid.to_string(),
                            "alias": alias,
                            "name": name,
                        })
                    })
                    .collect();
                tx.commit().await.map_err(crate::entity_pg::db_err)?;
                Ok(serde_json::Value::Array(out))
            }
        })
        .await
        // A catalog-load failure must surface, not masquerade as "no
        // catalogs" — the UI would silently hide catalog-scoped screens.
        .map_err(|e| ApiError::of_db(&e))?
        .as_array()
        .cloned()
        .unwrap_or_default();

    Ok(Json(ApiEnvelope {
        data: UserContext {
            principal_id: ctx.principal_id.to_string(),
            display_name: ctx.display_name.clone(),
            tenant_id: ctx.tenant_id.to_string(),
            locale: "ru".to_string(),
            timezone: "Europe/Moscow".to_string(),
            auth_strength: ctx.auth_strength.clone(),
            groups_revision: profile.ctx.groups_revision,
            capabilities: profile.capabilities,
            workspaces: vec!["default".to_string()],
            policy_revision: profile.policy_revision,
            csrf_token: ctx.csrf_token.clone(),
            ui_rules: profile.ui_rules,
            catalogs,
        },
        meta: meta(),
    }))
}

/// Extend a still-valid session (singleflight target for the API client).
/// Cookie-authenticated mutation → same CSRF contract as every other write:
/// without it a cross-site POST could silently keep a victim session alive.
#[post("/auth/refresh")]
pub async fn refresh(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    csrf_header: CsrfHeader,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(ApiError::from(Status::ServiceUnavailable));
    };
    let Some(cookie) = cookies.get("session") else {
        return Err(ApiError::from(Status::Unauthorized));
    };
    let Ok(sid) = cookie.value().parse::<uuid::Uuid>() else {
        return Err(ApiError::from(Status::Unauthorized));
    };
    let csrf = csrf_header.0;
    let v = exec
        .run(
            enginer_db_runtime::Deadline::in_secs(5),
            move |pool| async move {
                let row: Option<(Option<String>,)> = sqlx::query_as(
                    "SELECT csrf_token FROM sec.session
                     WHERE session_id = $1 AND ended_at IS NULL AND expires_at > now()",
                )
                .bind(sid)
                .fetch_optional(&pool)
                .await
                .map_err(|e| enginer_db_runtime::DbError::Other {
                    message: e.to_string(),
                })?;
                let Some((session_csrf,)) = row else {
                    return Ok(serde_json::json!({ "outcome": "expired" }));
                };
                if session_csrf.as_deref().unwrap_or_default() != csrf {
                    return Ok(serde_json::json!({ "outcome": "csrf_mismatch" }));
                }
                sqlx::query(
                    "UPDATE sec.session
                     SET expires_at = now() + ($2 || ' hours')::interval, last_seen_at = now()
                     WHERE session_id = $1 AND ended_at IS NULL AND expires_at > now()",
                )
                .bind(sid)
                .bind(SESSION_TTL_HOURS)
                .execute(&pool)
                .await
                .map_err(|e| enginer_db_runtime::DbError::Other {
                    message: e.to_string(),
                })?;
                Ok(serde_json::json!({ "outcome": "refreshed" }))
            },
        )
        .await
        .map_err(|_| ApiError::from(Status::ServiceUnavailable))?;
    match v["outcome"].as_str() {
        Some("refreshed") => Ok(Json(serde_json::json!({ "ok": true }))),
        Some("csrf_mismatch") => Err(ApiError::from(Status::Forbidden)),
        _ => Err(ApiError::from(Status::Unauthorized)),
    }
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    routes![login, logout, me_context, refresh]
}
