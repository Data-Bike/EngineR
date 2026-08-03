//! Check, Assessment, and Workflow API routes (docs/06 §8, docs/14 §8-10,
//! docs/13 §7-8).
//!
//! Phase 5 surface:
//! - `POST /api/v1/checks` — request a check (docs/14 §4);
//! - `GET /api/v1/checks/<id>` — get check status;
//! - `GET /api/v1/checks/<id>/evidence` — get check evidence;
//! - `GET /api/v1/entities/<id>/assessment` — get latest assessment;
//! - `POST /api/v1/entities/<id>/assessment` — trigger assessment;
//! - `GET /api/v1/entities/<id>/factors` — get risk factors;
//! - `POST /api/v1/workflows` — start workflow instance;
//! - `GET /api/v1/workflows/<id>` — get workflow status;
//! - `POST /api/v1/workflows/<id>/transition` — transition workflow;
//! - `GET /api/v1/workflows/<id>/tasks` — list workflow tasks;
//! - `POST /api/v1/tasks/<id>/complete` — complete task;
//! - `POST /api/v1/tasks/<id>/cancel` — cancel task.
//!
//! Security: `check.run` for checks, `entity.read` for assessment/factors,
//! `entity.create` for workflow start.

use crate::state::AppState;
use enginer_security_api::client::SecurityClient;
use enginer_security_api::contract::{
    ActionDecisionRequest, DecisionEffectDto, SecurityContextDto,
};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post, routes};
use serde::{Deserialize, Serialize};

// --- Check routes ---

/// Check request body (docs/14 §4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequestBody {
    pub target_entity_id: String,
    pub check_kind: String,
    pub connector_instance_id: Option<String>,
    pub parameters: serde_json::Value,
}

/// Check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResponse {
    pub check_id: uuid::Uuid,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Evidence response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceResponse {
    pub evidence: Vec<serde_json::Value>,
}

// --- Assessment routes ---

/// Assessment response (docs/14 §10).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentResponse {
    pub assessment_id: String,
    pub target_entity_id: String,
    pub final_score: f64,
    pub level: String,
    pub status: String,
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub factors: Vec<FactorSummary>,
}

/// Factor summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FactorSummary {
    pub factor_id: String,
    pub criterion_id: String,
    pub severity: String,
    pub status: String,
}

/// Trigger assessment request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerAssessmentBody {
    pub model_id: Option<String>,
    pub force: bool,
}

// --- Workflow routes ---

/// Start workflow request (docs/13 §7).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowBody {
    pub definition_id: String,
    pub target_entity_id: String,
    pub variables: serde_json::Value,
}

/// Workflow response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResponse {
    pub instance_id: String,
    pub definition_id: String,
    pub current_state: String,
    pub terminated: bool,
}

/// Transition request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionBody {
    pub to: String,
    pub trigger: String,
}

/// Task response (docs/13 §8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: String,
    pub title: String,
    pub kind: String,
    pub status: String,
    pub priority: String,
    pub assignee_id: Option<String>,
    pub due_date: chrono::DateTime<chrono::Utc>,
}

/// Complete task request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteTaskBody {
    pub result: serde_json::Value,
}

/// Cancel task request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelTaskBody {
    pub reason: String,
}

fn security_context(state: &AppState, purpose: &str) -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: uuid::Uuid::nil(),
        principal_id: uuid::Uuid::nil(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: purpose.to_string(),
        locale: "en-US".to_string(),
        time: chrono::Utc::now(),
        client_service: state.config.security.internal_mtls_identity.clone(),
    }
}

fn check_capability(
    state: &AppState,
    action: &str,
    purpose: &str,
) -> Result<(), rocket::http::Status> {
    if let Some(client) = &state.security_client {
        let ctx = security_context(state, purpose);
        let req = ActionDecisionRequest {
            context: ctx,
            action: action.to_string(),
            resource_kind: "entity".to_string(),
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

// --- Check endpoints ---

#[post("/checks", data = "<body>")]
pub async fn request_check(
    body: Json<CheckRequestBody>,
    state: &State<AppState>,
) -> Result<Json<CheckResponse>, rocket::http::Status> {
    check_capability(state, "check.run", "check_api")?;
    let _ = body.into_inner();
    Err(rocket::http::Status::NotImplemented)
}

#[get("/checks/<id>")]
pub async fn get_check(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<CheckResponse>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[get("/checks/<id>/evidence")]
pub async fn get_check_evidence(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<EvidenceResponse>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

// --- Assessment endpoints ---

#[get("/entities/<entity_id>/assessment")]
pub async fn get_assessment(
    entity_id: &str,
    _state: &State<AppState>,
) -> Result<Json<AssessmentResponse>, rocket::http::Status> {
    let _ = entity_id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/entities/<entity_id>/assessment", data = "<body>")]
pub async fn trigger_assessment(
    entity_id: &str,
    body: Json<TriggerAssessmentBody>,
    state: &State<AppState>,
) -> Result<Json<AssessmentResponse>, rocket::http::Status> {
    check_capability(state, "check.run", "assessment_api")?;
    let _ = (entity_id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[get("/entities/<entity_id>/factors")]
pub async fn get_factors(
    entity_id: &str,
    _state: &State<AppState>,
) -> Result<Json<Vec<FactorSummary>>, rocket::http::Status> {
    let _ = entity_id;
    Err(rocket::http::Status::NotImplemented)
}

// --- Workflow endpoints ---

#[post("/workflows", data = "<body>")]
pub async fn start_workflow(
    body: Json<StartWorkflowBody>,
    state: &State<AppState>,
) -> Result<Json<WorkflowResponse>, rocket::http::Status> {
    check_capability(state, "entity.create", "workflow_api")?;
    let _ = body.into_inner();
    Err(rocket::http::Status::NotImplemented)
}

#[get("/workflows/<id>")]
pub async fn get_workflow(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<WorkflowResponse>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/workflows/<id>/transition", data = "<body>")]
pub async fn transition_workflow(
    id: &str,
    body: Json<TransitionBody>,
    state: &State<AppState>,
) -> Result<Json<WorkflowResponse>, rocket::http::Status> {
    check_capability(state, "entity.edit", "workflow_api")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[get("/workflows/<id>/tasks")]
pub async fn list_workflow_tasks(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<Vec<TaskResponse>>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

// --- Task endpoints ---

#[post("/tasks/<id>/complete", data = "<body>")]
pub async fn complete_task(
    id: &str,
    body: Json<CompleteTaskBody>,
    state: &State<AppState>,
) -> Result<Json<TaskResponse>, rocket::http::Status> {
    check_capability(state, "entity.edit", "task_api")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[post("/tasks/<id>/cancel", data = "<body>")]
pub async fn cancel_task(
    id: &str,
    body: Json<CancelTaskBody>,
    state: &State<AppState>,
) -> Result<Json<TaskResponse>, rocket::http::Status> {
    check_capability(state, "entity.edit", "task_api")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

/// Check + Assessment + Workflow + Task routes collection.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        request_check,
        get_check,
        get_check_evidence,
        get_assessment,
        trigger_assessment,
        get_factors,
        start_workflow,
        get_workflow,
        transition_workflow,
        list_workflow_tasks,
        complete_task,
        cancel_task,
    ]
}
