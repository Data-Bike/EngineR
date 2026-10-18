//! Internal Rocket controllers exposing the Security API over mTLS
//! (docs/09 §2). Same contract as the local client (ADR-008).
//!
//! In production these are mounted under `/internal/security/v1` and reachable
//! only from the trusted network with mTLS + service allowlist. For Phase 0+1
//! we expose the routes; mTLS enforcement is a deployment concern.

use crate::client::SecurityClient;
use rocket::serde::json::Json;
use rocket::{get, post, Route, State};
use std::sync::Arc;

use crate::contract::{
    ActionDecisionRequest, ActionDecisionResponse, QueryDecisionRequest, QueryDecisionResponse,
};

/// The route list for `/internal/security/v1`.
pub fn routes() -> Vec<Route> {
    rocket::routes![decisions_action, decisions_query, policies_revision]
}

#[post("/decisions/action", data = "<req>")]
async fn decisions_action(
    req: Json<ActionDecisionRequest>,
    client: &State<Arc<dyn SecurityClient>>,
) -> Result<Json<ActionDecisionResponse>, rocket::http::Status> {
    client
        .action_decision(req.into_inner())
        .map(Json)
        .map_err(|_| rocket::http::Status::ServiceUnavailable)
}

#[post("/decisions/query", data = "<req>")]
async fn decisions_query(
    req: Json<QueryDecisionRequest>,
    client: &State<Arc<dyn SecurityClient>>,
) -> Result<Json<QueryDecisionResponse>, rocket::http::Status> {
    client
        .query_decision(req.into_inner())
        .map(Json)
        .map_err(|_| rocket::http::Status::ServiceUnavailable)
}

#[get("/policies/revision")]
async fn policies_revision() -> Json<serde_json::Value> {
    // Placeholder: real implementation reads the current published policy
    // revision from the policy store.
    Json(serde_json::json!({"policy_revision": 0}))
}

/// Convenience constructor for the local client used by the API app.
pub fn local_client(snapshot: Arc<dyn crate::client::SnapshotProvider>) -> Arc<dyn SecurityClient> {
    Arc::new(crate::client::LocalSecurityClient::new(snapshot))
}
