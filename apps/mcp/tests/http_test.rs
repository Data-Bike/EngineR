//! Route-level tests for the Rocket layer (auth wiring, method handling,
//! the agent-tool doc endpoint). The API is unreachable in these tests —
//! `McpState` points at a dead port, so a token validates only when a
//! context is pre-seeded into `state.contexts`.

use enginer_mcp::{McpState, AGENT_TOOL_MD};
use rocket::http::{Header, Status};
use rocket::local::asynchronous::Client;
use serde_json::json;
use std::time::Instant;

fn offline_state() -> McpState {
    // Nothing listens on port 1 — transport failure on any validation call.
    McpState::new("http://127.0.0.1:1")
}

async fn client(state: McpState) -> Client {
    Client::tracked(enginer_mcp::rocket(state)).await.unwrap()
}

fn valid_ctx() -> serde_json::Value {
    json!({ "principal_id": "p1", "capabilities": [] })
}

#[rocket::async_test]
async fn agent_tool_md_requires_bearer_header() {
    let client = client(offline_state()).await;
    let res = client.get("/agent-tool.md").dispatch().await;
    assert_eq!(res.status(), Status::Unauthorized);
}

#[rocket::async_test]
async fn agent_tool_md_rejects_unverifiable_token() {
    let client = client(offline_state()).await;
    let res = client
        .get("/agent-tool.md")
        .header(Header::new("Authorization", "Bearer egr_garbage"))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Unauthorized);
    let body: serde_json::Value = res.into_json().await.unwrap();
    assert!(body.get("error").is_some());
}

#[rocket::async_test]
async fn agent_tool_md_serves_embedded_skill_doc() {
    let state = offline_state();
    // Pre-seed a validated context so no API round-trip is needed.
    state
        .contexts
        .lock()
        .unwrap()
        .insert("egr_ok".to_string(), (Instant::now(), Some(valid_ctx())));
    let client = client(state).await;
    let res = client
        .get("/agent-tool.md")
        .header(Header::new("Authorization", "Bearer egr_ok"))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let ct = res.content_type().unwrap().to_string();
    assert!(
        ct.starts_with("text/markdown"),
        "unexpected content type {ct}"
    );
    assert!(ct.contains("charset=utf-8"), "missing charset in {ct}");
    assert_eq!(res.into_string().await.unwrap(), AGENT_TOOL_MD);
}

#[rocket::async_test]
async fn transport_failure_does_not_poison_token_cache() {
    let client = client(offline_state()).await;
    // API unreachable → 401, and crucially the failure must NOT be cached
    // (a cached `None` would keep rejecting the token for 30 s after the
    // API recovers).
    let res = client
        .get("/agent-tool.md")
        .header(Header::new("Authorization", "Bearer egr_t"))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Unauthorized);
    let state = client.rocket().state::<McpState>().unwrap();
    assert!(
        !state.contexts.lock().unwrap().contains_key("egr_t"),
        "transport failure was cached as a token verdict"
    );
    // API "recovers": the very next request validates normally.
    state
        .contexts
        .lock()
        .unwrap()
        .insert("egr_t".to_string(), (Instant::now(), Some(valid_ctx())));
    let res = client
        .get("/agent-tool.md")
        .header(Header::new("Authorization", "Bearer egr_t"))
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
}

#[rocket::async_test]
async fn mcp_get_is_405_health_is_open() {
    let client = client(offline_state()).await;
    assert_eq!(
        client.get("/mcp").dispatch().await.status(),
        Status::MethodNotAllowed
    );
    let res = client.get("/health").dispatch().await;
    assert_eq!(res.status(), Status::Ok);
    assert_eq!(
        res.into_json::<serde_json::Value>().await.unwrap(),
        json!({"status":"ok"})
    );
}

#[rocket::async_test]
async fn mcp_post_rejects_missing_and_bad_tokens() {
    let client = client(offline_state()).await;
    // No Authorization header at all.
    let res = client
        .post("/mcp")
        .header(rocket::http::ContentType::JSON)
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Unauthorized);
    // Bearer present but unverifiable → JSON-RPC-shaped 401.
    let res = client
        .post("/mcp")
        .header(rocket::http::ContentType::JSON)
        .header(Header::new("Authorization", "Bearer egr_bad"))
        .body(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Unauthorized);
    let body: serde_json::Value = res.into_json().await.unwrap();
    assert_eq!(body["jsonrpc"], "2.0");
    assert_eq!(body["error"]["code"], -32001);
}

#[rocket::async_test]
async fn mcp_post_ping_roundtrip_with_valid_token() {
    let state = offline_state();
    state
        .contexts
        .lock()
        .unwrap()
        .insert("egr_ok".to_string(), (Instant::now(), Some(valid_ctx())));
    let client = client(state).await;
    let res = client
        .post("/mcp")
        .header(rocket::http::ContentType::JSON)
        .header(Header::new("Authorization", "Bearer egr_ok"))
        .body(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#)
        .dispatch()
        .await;
    assert_eq!(res.status(), Status::Ok);
    let body: serde_json::Value = res.into_json().await.unwrap();
    assert_eq!(body["id"], 7);
    assert_eq!(body["result"], json!({}));
}
