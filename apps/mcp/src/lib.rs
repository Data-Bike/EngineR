//! EngineR MCP server — Streamable HTTP transport (MCP spec 2025-06-18).
//!
//! The server is a thin authenticated bridge: every request must carry
//! `Authorization: Bearer egr_…` (an EngineR API token). The token is
//! validated by calling `GET /api/v1/me/context` on the API; the returned
//! capability set filters `tools/list`, and `tools/call` forwards the same
//! token to the API so all PDP/PEP/data-permission enforcement stays
//! authoritative on the API side.
//!
//! Supported methods: `initialize`, `notifications/*` (202),
//! `ping`, `tools/list`, `tools/call`. Responses are single JSON-RPC
//! objects (`application/json`) — no SSE streaming.

use base64ct::Encoding;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

pub mod tools;
use tools::ToolDef;

/// MCP protocol revision advertised at `initialize`.
pub const PROTOCOL_VERSION: &str = "2025-06-18";
/// Agent-tooling documentation — `.devin/skills/enginer-mcp/SKILL.md` from
/// the repository, embedded at build time (the Dockerfile copies it into
/// the builder stage; it is not present in the runtime image).
pub const AGENT_TOOL_MD: &str = include_str!("../../../.devin/skills/enginer-mcp/SKILL.md");
/// How long a validated token context is cached.
const CONTEXT_CACHE_TTL: Duration = Duration::from_secs(30);
/// Hard cap on cached token contexts — an unauthenticated client could
/// otherwise grow the map forever with garbage bearer strings.
const CONTEXT_CACHE_MAX: usize = 4096;

/// Shared server state.
pub struct McpState {
    /// EngineR API base, e.g. `http://enginer-api:8010/api/v1`.
    pub api_base: String,
    pub http: reqwest::Client,
    /// token → (context json, fetched_at). `None` inside means a recent 401.
    pub contexts: Mutex<HashMap<String, (Instant, Option<Value>)>>,
}

impl McpState {
    pub fn new(api_base: &str) -> Self {
        Self {
            api_base: api_base.trim_end_matches('/').to_string(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
            contexts: Mutex::new(HashMap::new()),
        }
    }

    /// Validate the bearer token against the API and return the principal
    /// context (`/me/context` data). Cached briefly to keep `tools/list`
    /// cheap; `None` = token rejected.
    pub async fn validate_token(&self, token: &str) -> Option<Value> {
        if let Some((at, cached)) = self.contexts.lock().unwrap().get(token) {
            if at.elapsed() < CONTEXT_CACHE_TTL {
                return cached.clone();
            }
        }
        let res = self
            .http
            .get(format!("{}/me/context", self.api_base))
            .bearer_auth(token)
            .send()
            .await;
        let Ok(res) = res else {
            // Transport failure — not a verdict on the token. Caching `None`
            // here would mark every token invalid for the TTL during an API
            // blip, so bail without touching the cache.
            return None;
        };
        let ctx = if res.status().is_success() {
            res.json::<Value>()
                .await
                .ok()
                .and_then(|env| env.get("data").cloned())
        } else {
            None
        };
        {
            let mut cache = self.contexts.lock().unwrap();
            if cache.len() >= CONTEXT_CACHE_MAX {
                // Drop expired entries first; if the map is still full of
                // fresh (mostly junk) tokens, clear it — the cache is an
                // optimisation, correctness is unaffected.
                cache.retain(|_, (at, _)| at.elapsed() < CONTEXT_CACHE_TTL);
                if cache.len() >= CONTEXT_CACHE_MAX {
                    cache.clear();
                }
            }
            cache.insert(token.to_string(), (Instant::now(), ctx.clone()));
        }
        ctx
    }

    /// Execute one tool call against the EngineR API on behalf of the token.
    pub async fn call_tool(&self, token: &str, tool: &ToolDef, args: &Value) -> ToolOutcome {
        let prepared = match prepare_call(&self.api_base, tool, args) {
            Ok(p) => p,
            Err(msg) => return ToolOutcome::err(msg),
        };
        let mut req = match tool.method {
            "GET" => self.http.get(&prepared.url),
            "DELETE" => self.http.delete(&prepared.url),
            "POST" => self.http.post(&prepared.url),
            "PUT" => self.http.put(&prepared.url),
            "PATCH" => self.http.patch(&prepared.url),
            other => return ToolOutcome::err(format!("unsupported method {other}")),
        }
        .bearer_auth(token);
        if !prepared.query.is_empty() {
            req = req.query(&prepared.query);
        }
        if let Some(etag) = prepared.if_match {
            req = req.header("If-Match", etag);
        }
        if let Some(raw) = prepared.raw_body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
                .body(raw);
        } else if matches!(tool.method, "POST" | "PUT" | "PATCH") {
            req = req.json(&prepared.body);
        }
        match req.send().await {
            Ok(res) => {
                let status = res.status();
                let is_json = res
                    .headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .map(|ct| ct.contains("json"))
                    .unwrap_or(false);
                let payload = if is_json {
                    serde_json::from_str::<Value>(&res.text().await.unwrap_or_default())
                        .unwrap_or(Value::Null)
                } else {
                    let bytes = res.bytes().await.unwrap_or_default();
                    match std::str::from_utf8(&bytes) {
                        Ok(text) => serde_json::from_str::<Value>(text)
                            .unwrap_or_else(|_| Value::String(text.to_string())),
                        Err(_) => json!({
                            "content_base64": base64ct::Base64::encode_string(&bytes),
                            "size_bytes": bytes.len(),
                        }),
                    }
                };
                ToolOutcome {
                    is_error: !status.is_success(),
                    payload,
                    status: Some(status.as_u16()),
                }
            }
            Err(e) => ToolOutcome::err(format!("api call failed: {e}")),
        }
    }
}

/// A tool call translated into an HTTP request shape (no IO yet).
#[derive(Debug)]
pub struct PreparedCall {
    pub url: String,
    pub query: Vec<(String, String)>,
    pub body: Value,
    /// Decoded octet payload for tools that accept raw bytes (`data_base64`).
    pub raw_body: Option<Vec<u8>>,
    /// `If-Match` precondition extracted from the `if_match` argument.
    pub if_match: Option<String>,
}

/// Translate `tools/call` arguments into path/query/body parts. Pure — unit
/// tested without a server.
pub fn prepare_call(api_base: &str, tool: &ToolDef, args: &Value) -> Result<PreparedCall, String> {
    let mut path = tool.path.to_string();
    let mut query: Vec<(String, String)> = Vec::new();
    let mut body = serde_json::Map::new();
    let mut raw_body: Option<Vec<u8>> = None;
    let mut if_match: Option<String> = None;
    if let Some(obj) = args.as_object() {
        for (key, val) in obj {
            match tool.params.iter().find(|p| p.name == key) {
                Some(p) if p.loc == tools::ParamLoc::Path => {
                    let raw = match val {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        _ => return Err(format!("argument '{}' must be a string", p.name)),
                    };
                    // Path segments are ids/aliases — anything containing
                    // '/', '..', '?', '#' or '%' could reroute the call to
                    // a different endpoint after decoding/normalisation.
                    if raw.is_empty()
                        || raw.contains('/')
                        || raw.contains("..")
                        || raw.contains('?')
                        || raw.contains('#')
                        || raw.contains('%')
                    {
                        return Err(format!("invalid path argument '{}'", p.name));
                    }
                    path = path.replace(&format!("{{{}}}", p.name), &raw);
                }
                Some(p) if p.name == "data_base64" => {
                    let Some(b64) = val.as_str() else {
                        return Err("argument 'data_base64' must be a base64 string".into());
                    };
                    let bytes = base64ct::Base64::decode_vec(b64)
                        .map_err(|_| "argument 'data_base64' is not valid base64".to_string())?;
                    if bytes.is_empty() {
                        return Err("argument 'data_base64' is empty".into());
                    }
                    raw_body = Some(bytes);
                }
                Some(p) if p.name == "if_match" => {
                    if let Some(v) = val.as_str() {
                        if_match = Some(v.to_string());
                    }
                }
                Some(p) if p.loc == tools::ParamLoc::Query => {
                    if !val.is_null() {
                        let s = match val {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        };
                        query.push((p.name.to_string(), s));
                    }
                }
                Some(_) => {
                    body.insert(key.clone(), val.clone());
                }
                // Unknown argument: surfaced back to the caller instead of
                // silently dropped — the API would ignore it anyway.
                None => return Err(format!("unknown argument '{key}'")),
            }
        }
    }
    // `entities.update` takes a statement array on the wire; accept the
    // friendlier `{fields: {alias: value}}` object and translate it the same
    // way `pg_create_entity` normalises its `fields` map.
    if tool.name == "entities.update" {
        if let Some(Value::Object(map)) = body.get("fields").cloned() {
            let arr: Vec<Value> = map
                .into_iter()
                .map(|(k, v)| {
                    let val = if v.is_object() {
                        v
                    } else {
                        serde_json::json!({"kind": "text", "value": v})
                    };
                    serde_json::json!({"predicate_id": k, "value": val})
                })
                .collect();
            body.insert("fields".to_string(), Value::Array(arr));
        }
    }
    // Missing required params fail fast with a useful message.
    for p in tool.params {
        if p.required {
            let present = match p.loc {
                tools::ParamLoc::Path => !path.contains(&format!("{{{}}}", p.name)),
                _ => args.get(p.name).is_some_and(|v| !v.is_null()),
            };
            if !present {
                return Err(format!("missing required argument '{}'", p.name));
            }
        }
    }
    if path.contains('{') {
        return Err(format!("unresolved path placeholder in '{path}'"));
    }
    Ok(PreparedCall {
        url: format!("{}{}", api_base.trim_end_matches('/'), path),
        query,
        body: Value::Object(body),
        raw_body,
        if_match,
    })
}

/// Normalized tool call result.
pub struct ToolOutcome {
    pub is_error: bool,
    pub payload: Value,
    pub status: Option<u16>,
}

impl ToolOutcome {
    fn err(msg: String) -> Self {
        Self {
            is_error: true,
            payload: json!({ "error": msg }),
            status: None,
        }
    }
}

fn rpc_result(id: &Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.clone(), "result": result })
}

fn rpc_error(id: &Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.clone(),
        "error": { "code": code, "message": message },
    })
}

/// Handle one JSON-RPC message. `ctx` is the validated `/me/context` payload
/// of the caller's token. Returns `None` for notifications (HTTP 202).
pub async fn handle_rpc(state: &McpState, token: &str, ctx: &Value, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned().unwrap_or(Value::Null);
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    // Notifications carry no id — acknowledge silently.
    let is_notification = msg.get("id").is_none();

    match method {
        "initialize" if is_notification => None,
        "initialize" => Some(rpc_result(
            &id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "enginer-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "EngineR MCP bridge. Tools are filtered by the token's capabilities; calls run as the token's principal.",
            }),
        )),
        "ping" if is_notification => None,
        "ping" => Some(rpc_result(&id, json!({}))),
        "tools/list" if is_notification => None,
        "tools/list" => {
            let caps: Vec<String> = ctx
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|c| c.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            let tools = tools::tools_for_capabilities(&caps);
            Some(rpc_result(
                &id,
                json!({ "tools": tools.iter().map(|t| tools::tool_descriptor(t)).collect::<Vec<_>>() }),
            ))
        }
        "tools/call" => {
            let params = msg.get("params").cloned().unwrap_or(json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            let Some(tool) = tools::TOOLS.iter().find(|t| t.name == name) else {
                return if is_notification {
                    None
                } else {
                    Some(rpc_error(&id, -32602, &format!("unknown tool '{name}'")))
                };
            };
            // Pre-flight capability check for a clean error — the API still
            // enforces authoritatively on the proxied call.
            let caps: Vec<String> = ctx
                .get("capabilities")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|c| c.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();
            if let Some(cap) = tool.capability {
                let ok = caps.iter().any(|c| c == cap)
                    || (cap.starts_with("user.") && caps.iter().any(|c| c == "users.manage"));
                if !ok {
                    return if is_notification {
                        None
                    } else {
                        Some(rpc_error(
                            &id,
                            -32602,
                            &format!("capability '{cap}' not granted to this token"),
                        ))
                    };
                }
            }
            let out = state.call_tool(token, tool, &args).await;
            // A notification form of tools/call is still executed (the
            // side effect is requested) but must not produce a reply.
            if is_notification {
                return None;
            }
            let text = serde_json::to_string_pretty(&out.payload)
                .unwrap_or_else(|_| out.payload.to_string());
            Some(rpc_result(
                &id,
                json!({
                    "content": [{ "type": "text", "text": text }],
                    "structuredContent": out.payload,
                    "isError": out.is_error,
                }),
            ))
        }
        other => {
            if is_notification || other.starts_with("notifications/") {
                return None;
            }
            Some(rpc_error(
                &id,
                -32601,
                &format!("method not found '{other}'"),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// HTTP layer — Rocket routes. Kept in the library so integration tests can
// mount the exact production route set; `main.rs` only wires config + launch.
// ---------------------------------------------------------------------------

use rocket::http::{ContentType, Status};
use rocket::serde::json::Json;
use rocket::{delete, get, post, routes, State};

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
        return Ok(McpResponse::Unauthorized(Json(json!({
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
            return Ok(McpResponse::Json(Json(json!({
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
        return AgentDocResponse::Unauthorized(Json(json!({
            "error": "invalid or expired API token"
        })));
    }
    AgentDocResponse::Markdown((
        ContentType::parse_flexible("text/markdown; charset=utf-8").expect("valid media type"),
        AGENT_TOOL_MD.to_string(),
    ))
}

#[get("/health")]
fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

/// Build the Rocket instance with every MCP route mounted. `main` supplies
/// the configured state; tests supply one pointing at a stubbed API.
pub fn rocket(state: McpState) -> rocket::Rocket<rocket::Build> {
    rocket::build().manage(state).mount(
        "/",
        routes![mcp_post, mcp_get, mcp_delete, agent_tool_md, health],
    )
}
