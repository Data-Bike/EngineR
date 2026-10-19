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
    // `secure` is enabled automatically when Rocket detects TLS (docs/16 §13).
    c
}

const SESSION_TTL_HOURS: i64 = 12;

#[post("/auth/login", data = "<body>")]
pub async fn login(
    state: &State<AppState>,
    body: Json<LoginBody>,
    cookies: &CookieJar<'_>,
    client: ClientMeta,
) -> Result<Json<ApiEnvelope<LoginResponse>>, Status> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(Status::ServiceUnavailable);
    };
    let body = body.into_inner();
    let login_name = body.username.trim().to_lowercase();
    let client_ip = client.ip.clone();
    if login_name.is_empty() || body.password.is_empty() {
        return Err(Status::Unauthorized);
    }

    let cred = authn::find_credential(&exec, &login_name)
        .await
        .map_err(|_| Status::ServiceUnavailable)?
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
        Err(Status::Unauthorized)
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
        .ok_or(Status::InternalServerError)?;
    let principal: uuid::Uuid = cred["principal_id"]
        .as_str()
        .and_then(|s| s.parse().ok())
        .ok_or(Status::InternalServerError)?;

    let principal_row = authn::load_principal(&exec, tenant, principal)
        .await
        .map_err(|_| Status::ServiceUnavailable)?
        .ok_or(Status::Unauthorized)?;
    if principal_row.1 != "active" {
        return Err(Status::Unauthorized);
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
    .map_err(|_| Status::ServiceUnavailable)?;

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
pub async fn logout(state: &State<AppState>, cookies: &CookieJar<'_>) -> Json<serde_json::Value> {
    if let (Some(exec), Some(cookie)) = (state.db_executor.clone(), cookies.get("session")) {
        if let Ok(sid) = cookie.value().parse::<uuid::Uuid>() {
            let _ = authn::end_session(&exec, sid).await;
        }
    }
    cookies.remove(Cookie::from("session"));
    Json(serde_json::json!({ "ok": true }))
}

#[get("/me/context")]
pub async fn me_context(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
    bearer: BearerToken<'_>,
) -> Result<Json<ApiEnvelope<UserContext>>, Status> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(Status::ServiceUnavailable);
    };
    // Same two credential channels as the PEP guard: Bearer API token wins
    // over the session cookie when both are present.
    let ctx = if let Some(token) = bearer.0 {
        authn::resolve_bearer(&exec, token)
            .await
            .map_err(|_| Status::ServiceUnavailable)?
            .ok_or(Status::Unauthorized)?
    } else {
        let Some(cookie) = cookies.get("session") else {
            return Err(Status::Unauthorized);
        };
        let Ok(sid) = cookie.value().parse::<uuid::Uuid>() else {
            return Err(Status::Unauthorized);
        };
        authn::resolve_session(&exec, sid)
            .await
            .map_err(|_| Status::ServiceUnavailable)?
            .ok_or(Status::Unauthorized)?
    };
    let profile = authn::principal_profile(&exec, &ctx)
        .await
        .map_err(|_| Status::ServiceUnavailable)?
        .ok_or(Status::Unauthorized)?;

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
        },
        meta: meta(),
    }))
}

/// Extend a still-valid session (singleflight target for the API client).
#[post("/auth/refresh")]
pub async fn refresh(
    state: &State<AppState>,
    cookies: &CookieJar<'_>,
) -> Result<Json<serde_json::Value>, Status> {
    let Some(exec) = state.db_executor.clone() else {
        return Err(Status::ServiceUnavailable);
    };
    let Some(cookie) = cookies.get("session") else {
        return Err(Status::Unauthorized);
    };
    let Ok(sid) = cookie.value().parse::<uuid::Uuid>() else {
        return Err(Status::Unauthorized);
    };
    let v = exec
        .run(
            enginer_db_runtime::Deadline::in_secs(5),
            move |pool| async move {
                let res = sqlx::query(
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
                Ok(serde_json::json!({ "updated": res.rows_affected() }))
            },
        )
        .await
        .map_err(|_| Status::ServiceUnavailable)?;
    if v["updated"].as_u64().unwrap_or(0) == 0 {
        return Err(Status::Unauthorized);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    routes![login, logout, me_context, refresh]
}
