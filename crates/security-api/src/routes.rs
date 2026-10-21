//! Internal Rocket controllers exposing the Security API over mTLS
//! (docs/09 §2). Same contract as the local client (ADR-008).
//!
//! In production these are mounted under `/internal/security/v1` and reachable
//! only from the trusted network with mTLS + service allowlist. Until mTLS is
//! wired, the surface is protected by a shared-secret bearer
//! (`ENGINER_INTERNAL_TOKEN`) enforced in-process — the mount lives on the
//! public listener, so an unauthenticated request must never reach the PDP
//! with a caller-supplied `security_context` (capability oracle).

use crate::client::SecurityClient;
use rocket::request::{self, FromRequest, Request};
use rocket::serde::json::Json;
use rocket::{get, post, Route, State};
use std::sync::Arc;

use crate::contract::{
    ActionDecisionRequest, ActionDecisionResponse, QueryDecisionRequest, QueryDecisionResponse,
};

/// Shared-secret guard for `/internal/*`: requires
/// `Authorization: Bearer $ENGINER_INTERNAL_TOKEN`. Fails closed when the
/// variable is unset — no secret configured means no internal traffic.
pub struct InternalAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for InternalAuth {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let expected = std::env::var("ENGINER_INTERNAL_TOKEN").unwrap_or_default();
        let presented = req
            .headers()
            .get_one("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or_default();
        // Constant-time-ish comparison: both sides hashed before compare so
        // length/content do not leak through early-exit timing.
        let ok = !expected.is_empty()
            && sha256_hex(presented) == sha256_hex(&expected);
        if ok {
            request::Outcome::Success(InternalAuth)
        } else {
            request::Outcome::Error((rocket::http::Status::Unauthorized, ()))
        }
    }
}

fn sha256_hex(s: &str) -> [u8; 32] {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    h.update(s.as_bytes());
    h.finalize().into()
}

/// The route list for `/internal/security/v1`.
pub fn routes() -> Vec<Route> {
    rocket::routes![decisions_action, decisions_query, policies_revision]
}

#[post("/decisions/action", data = "<req>")]
async fn decisions_action(
    req: Json<ActionDecisionRequest>,
    client: &State<Arc<dyn SecurityClient>>,
    _auth: InternalAuth,
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
    _auth: InternalAuth,
) -> Result<Json<QueryDecisionResponse>, rocket::http::Status> {
    client
        .query_decision(req.into_inner())
        .map(Json)
        .map_err(|_| rocket::http::Status::ServiceUnavailable)
}

#[get("/policies/revision")]
async fn policies_revision(_auth: InternalAuth) -> Json<serde_json::Value> {
    // Placeholder: real implementation reads the current published policy
    // revision from the policy store.
    Json(serde_json::json!({"policy_revision": 0}))
}

/// Convenience constructor for the local client used by the API app.
pub fn local_client(snapshot: Arc<dyn crate::client::SnapshotProvider>) -> Arc<dyn SecurityClient> {
    Arc::new(crate::client::LocalSecurityClient::new(snapshot))
}
