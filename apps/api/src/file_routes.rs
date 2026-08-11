//! Files API routes (docs/15 §1-3).
//!
//! Phase 5 surface:
//! - `POST /api/v1/files/upload/init` — initialize staged upload;
//! - `PUT /api/v1/files/upload/<id>/chunks/<index>` — upload chunk;
//! - `POST /api/v1/files/upload/<id>/complete` — complete upload;
//! - `POST /api/v1/files/upload/<id>/abort` — abort upload;
//! - `GET /api/v1/files/upload/<id>` — get upload status;
//! - `GET /api/v1/files/<id>` — download file;
//! - `DELETE /api/v1/files/<id>` — delete file.
//!
//! Security: `bulk.import` for upload, `bulk.export` for download.

use crate::state::AppState;
use enginer_security_api::contract::{
    ActionDecisionRequest, DecisionEffectDto, SecurityContextDto,
};
use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put, routes};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Upload init request body (docs/15 §3 step 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInitBody {
    pub filename: String,
    pub content_type: String,
    pub expected_size: u64,
    pub expected_hash: String,
    pub hash_algorithm: String,
    pub chunk_size: u64,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default = "default_storage_class")]
    pub storage_class: String,
}

fn default_storage_class() -> String {
    "hot".to_string()
}

/// Upload init response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadInitResponse {
    pub upload_id: String,
    pub total_chunks: u32,
    pub status: String,
}

/// Upload complete request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadCompleteBody {
    pub final_hash: String,
    pub total_size: u64,
}

/// Upload abort request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadAbortBody {
    pub reason: String,
}

/// Upload status response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadStatusResponse {
    pub upload_id: String,
    pub status: String,
    pub progress: f64,
    pub received_chunks: u32,
    pub total_chunks: u32,
    pub object_id: Option<String>,
}

/// File metadata response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadataResponse {
    pub object_id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub content_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub classification: String,
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
            resource_kind: "file".to_string(),
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

#[post("/files/upload/init", data = "<body>")]
pub async fn upload_init(
    body: Json<UploadInitBody>,
    state: &State<AppState>,
) -> Result<Json<UploadInitResponse>, rocket::http::Status> {
    // bulk.import is high_risk + requires step_up (docs/08 §9)
    check_capability(state, "bulk.import", "file_upload")?;
    let _ = body.into_inner();
    Err(rocket::http::Status::NotImplemented)
}

#[put("/files/upload/<id>/chunks/<index>", data = "<chunk>")]
pub async fn upload_chunk(
    id: &str,
    index: u32,
    chunk: Vec<u8>,
    state: &State<AppState>,
) -> Result<Json<serde_json::Value>, rocket::http::Status> {
    check_capability(state, "bulk.import", "file_upload")?;
    let _ = (id, index, chunk);
    Err(rocket::http::Status::NotImplemented)
}

#[post("/files/upload/<id>/complete", data = "<body>")]
pub async fn upload_complete(
    id: &str,
    body: Json<UploadCompleteBody>,
    state: &State<AppState>,
) -> Result<Json<UploadStatusResponse>, rocket::http::Status> {
    check_capability(state, "bulk.import", "file_upload")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[post("/files/upload/<id>/abort", data = "<body>")]
pub async fn upload_abort(
    id: &str,
    body: Json<UploadAbortBody>,
    state: &State<AppState>,
) -> Result<rocket::http::Status, rocket::http::Status> {
    check_capability(state, "bulk.import", "file_upload")?;
    let _ = (id, body.into_inner());
    Err(rocket::http::Status::NotImplemented)
}

#[get("/files/upload/<id>")]
pub async fn upload_status(
    id: &str,
    _state: &State<AppState>,
) -> Result<Json<UploadStatusResponse>, rocket::http::Status> {
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

#[get("/files/<id>")]
pub async fn download_file(
    id: &str,
    state: &State<AppState>,
) -> Result<Json<FileMetadataResponse>, rocket::http::Status> {
    // bulk.export is high_risk + requires step_up (docs/08 §9)
    check_capability(state, "bulk.export", "file_download")?;
    let _ = id;
    // TODO: replace with real storage metadata once file persistence is wired.
    Ok(Json(FileMetadataResponse {
        object_id: id.to_string(),
        filename: format!("{id}.bin"),
        content_type: "application/octet-stream".to_string(),
        size: 0,
        content_hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        created_at: chrono::Utc::now(),
        classification: "unclassified".to_string(),
    }))
}

#[delete("/files/<id>")]
pub async fn delete_file(
    id: &str,
    state: &State<AppState>,
) -> Result<rocket::http::Status, rocket::http::Status> {
    check_capability(state, "bulk.import", "file_delete")?;
    let _ = id;
    Err(rocket::http::Status::NotImplemented)
}

/// File routes collection.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        upload_init,
        upload_chunk,
        upload_complete,
        upload_abort,
        upload_status,
        download_file,
        delete_file,
    ]
}
