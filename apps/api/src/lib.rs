//! EngineR Rocket JSON API (docs/02, docs/06).
//!
//! Phase 0+1 surface:
//! - `GET /health/live`, `GET /health/ready`;
//! - `GET /api/v1/version`;
//! - internal Security API under `/internal/security/v1` (mounted from
//!   `enginer_security_api::routes`).
//!
//! Phase 2 surface:
//! - Entity CRUD + query routes (`entity_routes`).
//!
//! Phase 5 surface:
//! - Schema admin routes (`schema_routes`): draft/validate/impact/publish/retire;
//! - Connector CRUD routes (`connector_routes`): list/create/update/enable/
//!   disable/health/delete;
//! - Check + Assessment + Workflow + Task routes (`check_routes`): request
//!   check, get assessment, trigger assessment, get factors, start/transition
//!   workflow, complete/cancel task;
//! - File routes (`file_routes`): staged upload init/chunk/complete/abort,
//!   download, delete;
//! - Audit routes (`audit_routes`): query entries, chain checkpoint, verify
//!   chain, retention policy.

#![forbid(unsafe_code)]

mod audit_routes;
mod auth_routes;
mod check_routes;
mod config;
mod connector_routes;
mod cors;
mod entity_routes;
mod file_routes;
mod health;
mod mock_routes;
mod schema_routes;
mod state;

pub use config::ApiConfig;
pub use state::AppState;

use rocket::serde::json::Json;
use rocket::{get, options, routes, Build, Rocket};

/// Build the Rocket application with the given state.
pub fn rocket_build(state: AppState) -> Rocket<Build> {
    let security_client = state.security_client.clone();

    // Use rocket::build() which reads Rocket.toml + ROCKET_* env vars
    // automatically. The secret_key comes from Rocket.toml or
    // ROCKET_SECRET_KEY env var.
    let mut rocket = rocket::build()
        .manage(state)
        .manage(mock_routes::MockDb::new())
        .attach(cors::CorsFairing)
        .mount("/health", routes![health::live, health::ready])
        .mount("/health", routes![options_preflight])
        .mount("/api/v1", routes![version])
        .mount("/api/v1", routes![options_preflight])
        .mount("/api/v1", auth_routes::routes())
        .mount("/api/v1", mock_routes::routes());

    if let Some(client) = security_client {
        rocket = rocket
            .manage(client)
            .mount("/internal/security/v1", enginer_security_api::routes::routes());
    }
    rocket
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
