//! EngineR MCP server — Streamable HTTP endpoint.
//!
//! Routes:
//! - `POST /mcp`          — JSON-RPC (initialize / tools/list / tools/call / ping).
//! - `GET  /mcp`          — 405: this server does not open SSE streams.
//! - `DELETE /mcp`        — 200: sessions are stateless, nothing to terminate.
//! - `GET  /agent-tool.md`— agent-tooling documentation (the enginer-mcp
//!   SKILL.md), served as `text/markdown` to any caller with a valid token.
//! - `GET  /health`       — liveness probe for Docker.
//!
//! Auth: `Authorization: Bearer egr_…` (EngineR API token) on every call.

use enginer_mcp::{handle_rpc, McpState, AGENT_TOOL_MD};
use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, State};
use serde_json::Value;

/// Extracted `Bearer` token (any value — validation happens against the API).
struct Bearer(String);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Bearer {
    type Error = &'static str;

    async fn from_request(
        req: &'r rocket::request::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        match req
            .headers()
            .get_one("authorization")
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
        {
            Some(t) => rocket::request::Outcome::Success(Bearer(t.to_string())),
            None => rocket::request::Outcome::Error((Status::Unauthorized, "missing bearer token")),
        }
    }
}

/// MCP responses: JSON-RPC results carry a body; notifications are acked
/// with bare 202 (no message body, per the streamable-HTTP spec).
#[derive(rocket::response::Responder)]
enum McpResponse {
    #[response(status = 200, content_type = "json")]
    Json(Json<Value>),
    #[response(status = 202)]
    Accepted(()),
    #[response(status = 401, content_type = "json")]
    Unauthorized(Json<Value>),
}

#[post("/mcp", data = "<body>")]
async fn mcp_post(
    state: &State<McpState>,
    bearer: Bearer,
    body: Json<Value>,
) -> Result<McpResponse, Status> {
    let Some(ctx) = state.validate_token(&bearer.0).await else {
        return Ok(McpResponse::Unauthorized(Json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": null,
            "error": { "code": -32001, "message": "invalid or expired API token" },
        }))));
    };
    let msg = body.into_inner();
    // Batch requests are supported: array in → array out.
    if let Value::Array(items) = msg {
        // JSON-RPC: an empty batch is an Invalid Request, and an
        // unbounded batch is a resource drain — cap it.
        if items.is_empty() || items.len() > 128 {
            return Ok(McpResponse::Json(Json(serde_json::json!({
                "jsonrpc": "2.0", "id": null,
                "error": { "code": -32600, "message": "invalid batch (empty or >128 messages)" },
            }))));
        }
        let mut out = Vec::new();
        for m in items {
            if let Some(r) = handle_rpc(state, &bearer.0, &ctx, &m).await {
                out.push(r);
            }
        }
        if out.is_empty() {
            return Ok(McpResponse::Accepted(()));
        }
        return Ok(McpResponse::Json(Json(Value::Array(out))));
    }
    match handle_rpc(state, &bearer.0, &ctx, &msg).await {
        Some(res) => Ok(McpResponse::Json(Json(res))),
        None => Ok(McpResponse::Accepted(())),
    }
}

#[get("/mcp")]
fn mcp_get() -> Status {
    // Server-initiated SSE streams are not implemented.
    Status::MethodNotAllowed
}

#[delete("/mcp")]
fn mcp_delete() -> Status {
    // Stateless — nothing to tear down.
    Status::Ok
}

/// Markdown doc vs. auth rejection for `GET /agent-tool.md`.
#[derive(rocket::response::Responder)]
enum AgentDocResponse {
    #[response(status = 200)]
    Markdown((ContentType, String)),
    #[response(status = 401, content_type = "json")]
    Unauthorized(Json<Value>),
}

/// `GET /agent-tool.md` — the SKILL.md an agent reads before driving this
/// server. Any valid token suffices: it is documentation, not domain data.
#[get("/agent-tool.md")]
async fn agent_tool_md(state: &State<McpState>, bearer: Bearer) -> AgentDocResponse {
    if state.validate_token(&bearer.0).await.is_none() {
        return AgentDocResponse::Unauthorized(Json(serde_json::json!({
            "error": "invalid or expired API token"
        })));
    }
    AgentDocResponse::Markdown((
        ContentType::new("text", "markdown"),
        AGENT_TOOL_MD.to_string(),
    ))
}

#[get("/health")]
fn health() -> Json<Value> {
    Json(serde_json::json!({ "status": "ok" }))
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    enginer_observability::init("info");

    // EngineR API base URL — inside docker it is `http://enginer-api:8010`.
    let base =
        std::env::var("ENGINER_API_URL").unwrap_or_else(|_| "http://localhost:8010".to_string());
    let api_base = format!("{}/api/v1", base.trim_end_matches('/'));
    tracing::info!(api_base = %api_base, "starting EngineR MCP server");

    rocket::build()
        .manage(McpState::new(&api_base))
        .mount("/", routes![mcp_post, mcp_get, mcp_delete, agent_tool_md, health])
        .launch()
        .await
        .map(|_| ())
}
