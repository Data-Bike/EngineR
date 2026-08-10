//! Schema admin API routes (docs/06 §5, docs/03 §7).
//!
//! Phase 5 surface:
//! - `GET /api/v1/schema/drafts` — list drafts;
//! - `GET /api/v1/schema/drafts/<id>` — get draft;
//! - `POST /api/v1/schema/drafts` — create draft;
//! - `POST /api/v1/schema/drafts/<id>/validate` — validate draft;
//! - `POST /api/v1/schema/drafts/<id>/impact` — impact analysis;
//! - `POST /api/v1/schema/drafts/<id>/publish` — publish draft;
//! - `POST /api/v1/schema/published/<id>/retire` — retire published.
//!
//! Security: `schema.draft` for draft/validate/impact, `schema.publish`
//! for publish (step-up required), `schema.publish` for retire.

use crate::state::AppState;
use enginer_security_api::contract::{
    ActionDecisionRequest, DecisionEffectDto, SecurityContextDto,
};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{get, post, routes};
use serde::{Deserialize, Serialize};

/// Create draft request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDraftBody {
    pub name: String,
    pub description: String,
    pub base_revision: Option<i64>,
}

/// Draft response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftResponse {
    pub draft_id: uuid::Uuid,
    pub name: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Validate response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Impact analysis response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactResponse {
    pub affected_entities: u64,
    pub breaking_changes: Vec<String>,
    pub migration_required: bool,
}

/// Publish response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResponse {
    pub revision_id: i64,
    pub published_at: chrono::DateTime<chrono::Utc>,
}

/// Retire request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetireSchemaBody {
    pub reason: String,
}

/// Build security context.
fn security_context(state: &AppState) -> SecurityContextDto {
    SecurityContextDto {
        request_id: uuid::Uuid::now_v7().to_string(),
        tenant_id: uuid::Uuid::nil(),
        principal_id: uuid::Uuid::nil(),
        session_id: uuid::Uuid::now_v7(),
        auth_strength: "single".to_string(),
        groups_revision: 1,
        purpose: "schema_admin".to_string(),
        locale: "en-US".to_string(),
        time: chrono::Utc::now(),
        client_service: state.config.security.internal_mtls_identity.clone(),
    }
}

/// Check capability via PEP.
fn check_capability(
    state: &AppState,
    action: &str,
) -> Result<(), rocket::http::Status> {
    if let Some(client) = &state.security_client {
        let ctx = security_context(state);
        let req = ActionDecisionRequest {
            context: ctx,
            action: action.to_string(),
            resource_kind: "schema".to_string(),
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

#[get("/schemas/drafts")]
pub async fn list_drafts(
    _state: &State<AppState>,
) -> Result<Json<Vec<DraftResponse>>, rocket::http::Status> {
    // Phase 5: route structure + PEP gate; persistence wired in full deployment.
    Err(rocket::http::Status::NotImplemented)
}

#[get("/schemas/drafts/<id>")]
pub async fn get_draft(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/schemas/drafts", data = "<body>")]
pub async fn create_draft(
    body: Json<CreateDraftBody>,
    state: &State<AppState>,
) -> Result<Json<DraftResponse>, rocket::http::Status> {
    check_capability(state, "schema.draft")?;
    let _ = body.into_inner();
    Err(rocket::http::Status::NotImplemented)
}

#[post("/schemas/drafts/<id>/validate")]
pub async fn validate_draft(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<ValidateResponse>, rocket::http::Status> {
    check_capability(state, "schema.draft")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/schemas/drafts/<id>/impact")]
pub async fn impact_draft(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<ImpactResponse>, rocket::http::Status> {
    check_capability(state, "schema.draft")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/schemas/drafts/<id>/publish")]
pub async fn publish_draft(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<PublishResponse>, rocket::http::Status> {
    // schema.publish requires step-up (docs/08 §9)
    check_capability(state, "schema.publish")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[post("/schemas/revisions/<id>/retire", data = "<body>")]
pub async fn retire_published(
    id: &str,
    body: Json<RetireSchemaBody>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    check_capability(state, "schema.publish")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

/// Schema routes collection.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        list_drafts,
        get_draft,
        create_draft,
        validate_draft,
        impact_draft,
        publish_draft,
        retire_published,
    ]
}
