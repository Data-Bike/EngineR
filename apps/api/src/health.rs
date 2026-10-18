//! Health endpoints (docs/21 §3).
//!
//! - `/health/live`: process alive, no external calls.
//! - `/health/ready`: config valid, DB usable, PDP/policy loaded.

use crate::state::AppState;
use rocket::get;
use rocket::serde::json::Json;

#[get("/live")]
pub fn live() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

#[get("/ready")]
pub async fn ready(state: &rocket::State<AppState>) -> Json<serde_json::Value> {
    let db_ok = state.db_executor.is_some();
    let security_ok = state.security_client.is_some();
    let ready = db_ok && security_ok;
    let status = if ready { "ready" } else { "degraded" };
    Json(serde_json::json!({
        "status": status,
        "db": db_ok,
        "security": security_ok,
    }))
}
