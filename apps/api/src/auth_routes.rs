//! Mock auth routes for local development / Docker demo.
//!
//! Provides:
//! - `POST /api/v1/auth/login` — mock login (any username/password works);
//! - `GET /api/v1/me/context` — mock user context with all capabilities;
//! - `POST /api/v1/auth/logout` — mock logout.
//!
//! In production these are handled by a real auth provider (docs/08).

use rocket::serde::json::Json;
use rocket::{get, post, routes};
use serde::{Deserialize, Serialize};

/// Login request body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginBody {
    pub username: String,
    pub password: String,
}

/// Login response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginResponse {
    pub principal_id: String,
    pub display_name: String,
    pub tenant_id: String,
}

/// User context (docs/16 §5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub principal_id: String,
    pub display_name: String,
    pub tenant_id: String,
    pub locale: String,
    pub timezone: String,
    pub auth_strength: String,
    pub groups_revision: i64,
    pub capabilities: Vec<String>,
    pub workspaces: Vec<String>,
    pub policy_revision: i64,
}

/// API envelope wrapper.
#[derive(Debug, Clone, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: serde_json::Value,
}

fn default_context(username: &str) -> UserContext {
    UserContext {
        principal_id: "00000000-0000-0000-0000-000000000001".to_string(),
        display_name: username.to_string(),
        tenant_id: "00000000-0000-0000-0000-000000000002".to_string(),
        locale: "ru".to_string(),
        timezone: "Europe/Moscow".to_string(),
        auth_strength: "normal".to_string(),
        groups_revision: 1,
        capabilities: vec![
            "admin.view".to_string(),
            "audit.view".to_string(),
            "audit.admin".to_string(),
            "entity.read".to_string(),
            "entity.create".to_string(),
            "entity.edit".to_string(),
            "check.run".to_string(),
            "connector.admin".to_string(),
            "schema.admin".to_string(),
            "risk.view".to_string(),
            "assessment.view".to_string(),
        ],
        workspaces: vec!["default".to_string()],
        policy_revision: 1,
    }
}

#[post("/auth/login", data = "<body>")]
pub async fn login(body: Json<LoginBody>) -> Json<ApiEnvelope<LoginResponse>> {
    let body = body.into_inner();
    Json(ApiEnvelope {
        data: LoginResponse {
            principal_id: "00000000-0000-0000-0000-000000000001".to_string(),
            display_name: body.username.clone(),
            tenant_id: "00000000-0000-0000-0000-000000000002".to_string(),
        },
        meta: serde_json::json!({ "request_id": "mock" }),
    })
}

#[post("/auth/logout")]
pub async fn logout() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

#[get("/me/context")]
pub async fn me_context() -> Json<ApiEnvelope<UserContext>> {
    Json(ApiEnvelope {
        data: default_context("admin"),
        meta: serde_json::json!({ "request_id": "mock" }),
    })
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    routes![login, logout, me_context]
}
