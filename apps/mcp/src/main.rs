//! `EngineR` MCP server — Streamable HTTP endpoint.
//!
//! Routes (see `lib.rs` for implementations):
//! - `POST /mcp`          — JSON-RPC (initialize / tools/list / tools/call / ping).
//! - `GET  /mcp`          — 405: this server does not open SSE streams.
//! - `DELETE /mcp`        — 200: sessions are stateless, nothing to terminate.
//! - `GET  /agent-tool.md`— agent-tooling documentation (the enginer-mcp
//!   SKILL.md), served as `text/markdown` to any caller with a valid token.
//! - `GET  /health`       — liveness probe for Docker.
//!
//! Auth: `Authorization: Bearer egr_…` (`EngineR` API token) on every call.

use enginer_mcp::McpState;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    enginer_observability::init("info");

    // EngineR API base URL — inside docker it is `http://enginer-api:8010`.
    let base =
        std::env::var("ENGINER_API_URL").unwrap_or_else(|_| "http://localhost:8010".to_string());
    let api_base = format!("{}/api/v1", base.trim_end_matches('/'));
    tracing::info!(api_base = %api_base, "starting `EngineR` MCP server");

    enginer_mcp::rocket(McpState::new(&api_base))
        .launch()
        .await
        .map(|_| ())
}
