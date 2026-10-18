//! EngineR Rocket JSON API (docs/02, docs/06).
//!
//! Phase 0+1 surface:
//! - `GET /health/live`, `GET /health/ready`;
//! - `GET /api/v1/version`;
//! - internal Security API under `/internal/security/v1` (mounted from
//!   `enginer_security_api::routes`).
//!
//! The public application surface is mounted through two stateful modules:
//! - `schema_admin_routes` for draft/validate/impact/publish/retire;
//! - `workspace_routes` for entities, catalog CRUD, graph, checks, risks,
//!   workflows, tasks, files, reports, connectors, permissions, and audit.
//! PostgreSQL-backed domain ports remain available for distributed deployments.

#![forbid(unsafe_code)]

mod audit_pg;
mod auth_routes;
pub(crate) mod authn;
mod config;
mod cors;
pub(crate) mod dataperm;
mod entity_pg;
mod files_pg;
mod health;
mod schema_admin_routes;
pub mod seed;
mod skr_pg;
mod state;
mod users_pg;
mod workspace_routes;

pub use config::ApiConfig;
pub use state::AppState;

use rocket::serde::json::Json;
use rocket::{catch, catchers, get, options, routes, Build, Rocket};

/// Build the Rocket application with the given state.
pub fn rocket_build(state: AppState) -> Rocket<Build> {
    let security_client = state.security_client.clone();

    // Use rocket::build() which reads Rocket.toml + ROCKET_* env vars
    // automatically. The secret_key comes from Rocket.toml or
    // ROCKET_SECRET_KEY env var.
    let mut rocket = rocket::build()
        .manage(state)
        .attach(cors::CorsFairing)
        .attach(AuditFairing)
        .mount("/health", routes![health::live, health::ready])
        .mount("/health", routes![options_preflight])
        .mount("/api/v1", routes![version])
        .mount("/api/v1", routes![options_preflight])
        .mount("/api/v1", auth_routes::routes())
        .mount("/api/v1", schema_admin_routes::routes())
        .mount("/api/v1", entity_pg::routes())
        .mount("/api/v1", audit_pg::routes())
        .mount("/api/v1", skr_pg::routes())
        .mount("/api/v1", files_pg::routes())
        .mount("/api/v1", users_pg::routes())
        .register(
            "/api/v1",
            catchers![
                api_bad_request,
                api_unauthorized,
                api_forbidden,
                api_not_found,
                api_unprocessable,
                api_internal_error,
            ],
        );

    if let Some(client) = security_client {
        rocket = rocket.manage(client).mount(
            "/internal/security/v1",
            enginer_security_api::routes::routes(),
        );
    }
    rocket
}

/// Tamper-evident audit for mutating API calls (docs/08 §11): after a
/// POST/PUT/PATCH/DELETE under `/api/v1` completes, append an `audit.event`
/// row attributed to the session principal resolved by `CapabilityGuard`.
/// Denied requests are already recorded inside the guard; this covers the
/// handler outcome (success/failure) for every other mutation.
struct AuditFairing;

#[rocket::async_trait]
impl rocket::fairing::Fairing for AuditFairing {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Mutation audit",
            kind: rocket::fairing::Kind::Response,
        }
    }

    async fn on_response<'r>(&self, req: &'r rocket::Request<'_>, res: &mut rocket::Response<'r>) {
        use rocket::http::Method;
        let path = req.uri().path().as_str().to_string();
        // Entity-detail views are logged so the user-activity screen can show
        // "objects the user viewed". Reads of collections stay unaudited.
        let is_entity_view = req.method() == Method::Get
            && path
                .strip_prefix("/api/v1/entities/")
                .is_some_and(|rest| rest.parse::<uuid::Uuid>().is_ok());
        if matches!(req.method(), Method::Get | Method::Head | Method::Options) && !is_entity_view {
            return;
        }
        if !path.starts_with("/api/v1/") || path == "/api/v1/auth/login" {
            return; // login/logout are audited with richer context in auth_routes
        }
        let Some(ctx) = req.local_cache(|| None::<authn::AuthContext>).clone() else {
            return;
        };
        let Some(state) = req.rocket().state::<AppState>() else {
            return;
        };
        let Some(exec) = state.db_executor.clone() else {
            return;
        };
        let status = res.status();
        let outcome = if status.code < 400 {
            "allowed"
        } else {
            "denied"
        };
        let action = format!("{} {}", req.method().as_str(), path);
        rocket::tokio::spawn(audit_pg::record_audit_event(
            exec,
            ctx.tenant_id,
            Some(ctx.principal_id),
            Some(ctx.session_id),
            "http_request",
            action,
            "http",
            None,
            Some("allow"),
            outcome,
            None,
            serde_json::json!({
                "status": status.code,
                "public_id": path
                    .strip_prefix("/api/v1/entities/")
                    .filter(|rest| rest.parse::<uuid::Uuid>().is_ok())
                    .map(str::to_string),
            }),
        ));
    }
}

fn api_error(code: &str, message: &str) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "error": {
            "code": code,
            "message_key": format!("errors.{code}"),
            "request_id": uuid::Uuid::now_v7().to_string(),
            "message": message,
        }
    }))
}

#[catch(400)]
fn api_bad_request() -> Json<serde_json::Value> {
    api_error("bad_request", "Некорректный запрос")
}

#[catch(401)]
fn api_unauthorized() -> Json<serde_json::Value> {
    api_error("unauthorized", "Требуется аутентификация")
}

#[catch(403)]
fn api_forbidden() -> Json<serde_json::Value> {
    api_error("forbidden", "Доступ запрещён")
}

#[catch(404)]
fn api_not_found() -> Json<serde_json::Value> {
    api_error("not_found", "Ресурс не найден")
}

#[catch(422)]
fn api_unprocessable() -> Json<serde_json::Value> {
    api_error("unprocessable_entity", "Запрос не прошёл валидацию")
}

#[catch(500)]
fn api_internal_error() -> Json<serde_json::Value> {
    api_error("internal_error", "Внутренняя ошибка сервера")
}

#[get("/version")]
fn version() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "enginer-api",
        "version": env!("CARGO_PKG_VERSION"),
        "api_version": "v1",
    }))
}

/// Catch-all OPTIONS handler for CORS preflight (docs/16 §13).
/// Returns 204 No Content; CORS headers are added by the CorsFairing.
#[options("/<_..>")]
fn options_preflight() -> rocket::http::Status {
    rocket::http::Status::NoContent
}
