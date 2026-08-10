//! Connector API routes (docs/06 §7, docs/14 §4).
//!
//! Phase 5 surface:
//! - `GET /api/v1/connectors` — list connector instances;
//! - `GET /api/v1/connectors/<id>` — get connector instance;
//! - `POST /api/v1/connectors` — create connector instance;
//! - `PATCH /api/v1/connectors/<id>` — update connector instance;
//! - `POST /api/v1/connectors/<id>/enable` — enable connector;
//! - `POST /api/v1/connectors/<id>/disable` — disable connector;
//! - `POST /api/v1/connectors/<id>/health` — health check;
//! - `DELETE /api/v1/connectors/<id>` — remove connector instance.
//!
//! Security: `connector.manage` capability.

use crate::state::AppState;
use enginer_security_api::contract::{
    ActionDecisionRequest, DecisionEffectDto, SecurityContextDto,
};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, patch, post, routes};
use serde::{Deserialize, Serialize};

/// Connector instance response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorInstanceResponse {
    pub instance_id: uuid::Uuid,
    pub connector_type_id: String,
    pub tenant_id: String,
    pub enabled: bool,
    pub environment: String,
    pub rate_limit_rps: u32,
    pub trust_rank: u8,
}

/// Create connector request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConnectorBody {
    pub connector_type_id: String,
    pub environment: String,
    pub base_url: Option<String>,
    pub rate_limit_rps: Option<u32>,
    pub concurrency: Option<u32>,
    pub timeout_seconds: Option<u32>,
    pub trust_rank: Option<u8>,
}

/// Update connector request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateConnectorBody {
    pub enabled: Option<bool>,
    pub rate_limit_rps: Option<u32>,
    pub concurrency: Option<u32>,
    pub timeout_seconds: Option<u32>,
}

/// Health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResponse {
    pub healthy: bool,
    pub latency_ms: u64,
    pub message: String,
}

fn security_context(state: &AppState) -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: uuid::Uuid::nil(),
        principal_id: uuid::Uuid::nil(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: "connector_admin".to_string(),
        locale: "en-US".to_string(),
        time: chrono::Utc::now(),
        client_service: state.config.security.internal_mtls_identity.clone(),
    }
}

fn check_capability(
    state: &AppState,
    action: &str,
) -> Result<(), rocket::http::Status> {
    if let Some(client) = &state.security_client {
        let ctx = security_context(state);
        let req = ActionDecisionRequest {
            context: ctx,
            action: action.to_string(),
            resource_kind: "connector".to_string(),
            resource_id: None,
            parameters: serde_json::Map::new(),
        };
        let dec = client
            .action_decision(req)
            .map_err(|_| rocket::http::Status::ServiceUnavailable)?;
        if !matches!(
            dec.effect,
            DecisionEffectDto::Allow | DecisionEffectDto::AllowWithObligations
        ) {
            return Err(rocket::http::Status::Forbidden);
        }
    }
    Ok(())
}

#[get("/connectors")]
pub async fn list_connectors(
    _state: &State<AppState>,
) -> Result<Json<Vec<ConnectorInstanceResponse>>, rocket::http::Status> {
    Err(rocket::http::Status::NotImplemented)
}

#[get("/connectors/<id>")]
pub async fn get_connector(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<ConnectorInstanceResponse>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/connectors", data = "<body>")]
pub async fn create_connector(
    body: Json<CreateConnectorBody>,
    state: &State<AppState>,
) -> Result<Json<ConnectorInstanceResponse>, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = body.into_inner();
    Err(rocket::http::Status::NotImplemented)
}

#[patch("/connectors/<id>", data = "<body>")]
pub async fn update_connector(
    id: &str,
    body: Json<UpdateConnectorBody>,
    state: &State<AppState>,
) -> Result<Json<ConnectorInstanceResponse>, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[post("/connectors/<id>/enable")]
pub async fn enable_connector(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/connectors/<id>/disable")]
pub async fn disable_connector(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/connectors/<id>/health")]
pub async fn health_check_connector(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<HealthCheckResponse>, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[delete("/connectors/<id>")]
pub async fn delete_connector(
    id: &str,
    state: &State<AppState>,
) -> Result<rocket::http::Status, rocket::http::Status> {
    check_capability(state, "connector.manage")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

/// Connector routes collection.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        list_connectors,
        get_connector,
        create_connector,
        update_connector,
        enable_connector,
        disable_connector,
        health_check_connector,
        delete_connector,
    ]
}
