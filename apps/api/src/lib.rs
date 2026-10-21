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
mod catalog_routes;
mod config;
mod cors;
pub(crate) mod dataperm;
mod dump_routes;
mod entity_pg;
mod field_kinds;
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

/// Fail any dump/restore runs left marked `running`/`restoring` by a
/// previous process. Called once at boot; see `dump_routes::sweep_dead_markers`.
pub fn sweep_orphaned_dump_runs() {
    dump_routes::sweep_dead_markers();
}

use rocket::serde::json::Json;
use rocket::{catch, catchers, get, options, routes, Build, Rocket};

/// Build the Rocket application with the given state.
pub fn rocket_build(state: AppState) -> Rocket<Build> {
    let security_client = state.security_client.clone();

    // Use rocket::build() which reads Rocket.toml + ROCKET_* env vars
    // automatically. The secret_key comes from Rocket.toml or
    // ROCKET_SECRET_KEY env var.
    let cors = cors::CorsFairing::new(&state.config.http.public_origin);
    let mut rocket = rocket::build()
        .manage(state)
        .attach(cors)
        .attach(AuditFairing)
        .mount("/health", routes![health::live, health::ready])
        .mount("/health", routes![options_preflight])
        .mount("/api/v1/health", routes![health::live, health::ready])
        .mount("/api/v1", routes![version])
        .mount("/api/v1", routes![options_preflight])
        .mount("/api/v1", auth_routes::routes())
        .mount("/api/v1", schema_admin_routes::routes())
        .mount("/api/v1", catalog_routes::routes())
        .mount("/api/v1", entity_pg::routes())
        .mount("/api/v1", audit_pg::routes())
        .mount("/api/v1", skr_pg::routes())
        .mount("/api/v1", files_pg::routes())
        .mount("/api/v1", users_pg::routes())
        .mount("/api/v1", dump_routes::routes())
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
        // Entity-scoped requests get `resource_kind=entity` + the real
        // resource_id so insert_event can tag the event with the entity's
        // catalog (docs/28 §8) and audit.query can filter by catalog.
        let entity_pid = path
            .strip_prefix("/api/v1/entities/")
            .and_then(|rest| rest.split('/').next())
            .and_then(|seg| seg.parse::<uuid::Uuid>().ok());
        rocket::tokio::spawn(audit_pg::record_audit_event(
            exec,
            ctx.tenant_id,
            Some(ctx.principal_id),
            Some(ctx.session_id),
            "http_request",
            action,
            if entity_pid.is_some() {
                "entity"
            } else {
                "http"
            },
            entity_pid,
            Some("allow"),
            outcome,
            None,
            serde_json::json!({
                "status": status.code,
                "public_id": entity_pid.map(|u| u.to_string()),
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

/// Route error responder carrying the machine-readable `error.code` —
/// same envelope shape as the top-level catchers but preserving the
/// specific code (`catalog.unknown`, `cursor.invalid`, …) instead of the
/// generic per-status one. `http_err` encodes `HTTP{status}:{detail}`
/// inside `DbError`; `of_db` decodes it back.
#[derive(Debug)]
pub struct ApiError {
    pub status: rocket::http::Status,
    pub code: String,
    pub message: String,
}

impl ApiError {
    fn default_code(status: rocket::http::Status) -> &'static str {
        match status.code {
            400 => "bad_request",
            401 => "unauthorized",
            403 => "forbidden",
            404 => "not_found",
            409 => "conflict",
            422 => "unprocessable_entity",
            _ => "internal_error",
        }
    }

    pub(crate) fn default_message(status: rocket::http::Status) -> &'static str {
        match status.code {
            400 => "Некорректный запрос",
            401 => "Требуется аутентификация",
            403 => "Доступ запрещён",
            404 => "Ресурс не найден",
            409 => "Конфликт версии или состояния",
            422 => "Запрос не прошёл валидацию",
            _ => "Внутренняя ошибка сервера",
        }
    }

    /// The message when it carries specifics beyond the generic per-status
    /// text — clients localize `message_key` and append this detail.
    pub fn detail(&self) -> Option<&str> {
        let msg = self.message.trim();
        (!msg.is_empty() && msg != Self::default_message(self.status)).then_some(msg)
    }

    /// Structured error with an explicit machine code (`cursor.invalid`,
    /// `catalog.denied`, …) and the generic per-status message.
    pub fn coded(status: rocket::http::Status, code: &str) -> Self {
        Self {
            status,
            code: code.to_string(),
            message: Self::default_message(status).to_string(),
        }
    }

    /// Decode a `DbError` produced by `http_err` (`HTTP{status}:{detail}`)
    /// or `sf_err` (`SF|{status}|{code}|{message}`). A detail that looks
    /// like a machine token (`catalog.unknown`) becomes `code`; free text
    /// becomes `message` under the generic per-status code.
    pub fn of_db(e: &enginer_db_runtime::DbError) -> Self {
        if let enginer_db_runtime::DbError::Other { message } = e {
            if let Some(rest) = message.strip_prefix("SF|") {
                let mut parts = rest.splitn(3, '|');
                let status = parts
                    .next()
                    .and_then(|s| s.parse::<u16>().ok())
                    .and_then(rocket::http::Status::from_code)
                    .unwrap_or(rocket::http::Status::InternalServerError);
                if status.code >= 500 {
                    tracing::error!(error = %e, status = %status, "sf error mapped to status");
                }
                return Self {
                    status,
                    code: parts.next().unwrap_or("internal_error").to_string(),
                    message: parts.next().unwrap_or("error").to_string(),
                };
            }
            if let Some(rest) = message.strip_prefix("HTTP") {
                if let Some((status_s, detail)) = rest.split_once(':') {
                    if let Some(status) = status_s
                        .parse::<u16>()
                        .ok()
                        .and_then(rocket::http::Status::from_code)
                    {
                        if status.code >= 500 {
                            tracing::error!(error = %e, status = %status, "db error mapped to status");
                        }
                        // Dotted/underscored single tokens are machine
                        // codes (`catalog.unknown`); bare words (`group`)
                        // stay human messages under the generic code.
                        let machine_token = !detail.is_empty()
                            && (detail.contains('.') || detail.contains('_'))
                            && detail.chars().all(|c| {
                                c.is_ascii_lowercase()
                                    || c.is_ascii_digit()
                                    || matches!(c, '.' | '_' | '-')
                            });
                        return if machine_token {
                            Self {
                                status,
                                code: detail.to_string(),
                                message: Self::default_message(status).to_string(),
                            }
                        } else {
                            Self {
                                status,
                                code: Self::default_code(status).to_string(),
                                message: detail.to_string(),
                            }
                        };
                    }
                }
            }
        }
        tracing::error!(error = %e, "unhandled db error");
        Self::from(rocket::http::Status::InternalServerError)
    }
}

impl From<rocket::http::Status> for ApiError {
    fn from(status: rocket::http::Status) -> Self {
        if status.code >= 500 {
            tracing::error!(status = %status, "api error mapped to 5xx");
        }
        Self {
            status,
            code: Self::default_code(status).to_string(),
            message: Self::default_message(status).to_string(),
        }
    }
}

impl<'r> rocket::response::Responder<'r, 'static> for ApiError {
    fn respond_to(
        self,
        request: &'r rocket::Request<'_>,
    ) -> rocket::response::Result<'static> {
        let mut body = serde_json::json!({
            "error": {
                "code": self.code,
                "message_key": format!("errors.{}", self.code),
                "request_id": uuid::Uuid::now_v7().to_string(),
                "message": self.message,
            }
        });
        if let Some(detail) = self.detail() {
            body["error"]["detail"] = serde_json::Value::String(detail.to_string());
        }
        let mut response = Json(body).respond_to(request)?;
        response.set_status(self.status);
        Ok(response)
    }
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

#[cfg(test)]
mod api_error_tests {
    use super::ApiError;
    use enginer_db_runtime::DbError;
    use rocket::http::Status;

    fn other(msg: &str) -> DbError {
        DbError::Other {
            message: msg.to_string(),
        }
    }

    #[test]
    fn of_db_decodes_http_machine_code() {
        let e = ApiError::of_db(&other("HTTP422:catalog.unknown"));
        assert_eq!(e.status, Status::UnprocessableEntity);
        assert_eq!(e.code, "catalog.unknown");
    }

    #[test]
    fn of_db_decodes_sf_envelope() {
        let e = ApiError::of_db(&other("SF|422|cursor.invalid|bad cursor"));
        assert_eq!(e.status, Status::UnprocessableEntity);
        assert_eq!(e.code, "cursor.invalid");
        assert_eq!(e.message, "bad cursor");
    }

    #[test]
    fn of_db_keeps_free_text_as_message() {
        let e = ApiError::of_db(&other("HTTP404:group not found"));
        assert_eq!(e.status, Status::NotFound);
        assert_eq!(e.code, "not_found");
        assert_eq!(e.message, "group not found");
    }

    #[test]
    fn detail_only_when_message_is_specific() {
        assert_eq!(
            ApiError::of_db(&other("HTTP422:required fields missing: name")).detail(),
            Some("required fields missing: name")
        );
        assert_eq!(ApiError::of_db(&other("HTTP422:due_date.invalid")).detail(), None);
        assert_eq!(ApiError::from(Status::NotFound).detail(), None);
        assert_eq!(
            ApiError::of_db(&other("SF|409|catalog.retired|Каталог погашен")).detail(),
            Some("Каталог погашен")
        );
    }

    #[test]
    fn of_db_unrecognised_falls_back_to_500() {
        let e = ApiError::of_db(&other("some driver failure"));
        assert_eq!(e.status, Status::InternalServerError);
    }
}
