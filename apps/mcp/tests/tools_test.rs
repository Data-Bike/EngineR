//! Unit tests for the MCP tool catalog + call preparation (no server).

use enginer_mcp::tools::{tools_for_capabilities, TOOLS};
use enginer_mcp::{handle_rpc, prepare_call, McpState};
use serde_json::json;
use std::collections::HashSet;

#[test]
fn tool_names_are_unique() {
    let names: HashSet<_> = TOOLS.iter().map(|t| t.name).collect();
    assert_eq!(names.len(), TOOLS.len(), "duplicate tool names");
}

#[test]
fn every_tool_covers_whole_api_shape() {
    // Every path must be /api/v1-relative and placeholders must be declared.
    for t in TOOLS {
        assert!(t.path.starts_with('/'), "{}: path not rooted", t.name);
        for p in t.params {
            if p.loc == enginer_mcp::tools::ParamLoc::Path {
                assert!(
                    t.path.contains(&format!("{{{}}}", p.name)),
                    "{}: path param '{}' missing from template",
                    t.name,
                    p.name
                );
            }
        }
        for (i, seg) in t.path.split('{').skip(1).enumerate() {
            let name = seg.split('}').next().unwrap();
            assert!(
                t.params.iter().any(|p| p.name == name),
                "{}: placeholder '{}' not declared (seg {i})",
                t.name,
                name
            );
        }
    }
}

#[test]
fn tools_filter_by_capability() {
    let viewer = vec!["entity.read".to_string()];
    let names: Vec<_> = tools_for_capabilities(&viewer)
        .iter()
        .map(|t| t.name)
        .collect();
    assert!(names.contains(&"entities.list"));
    assert!(names.contains(&"context.get")); // capability: None
    assert!(!names.contains(&"users.create"));
    assert!(!names.contains(&"audit.query"));
    assert!(!names.contains(&"schema.draft_publish"));

    let admin = vec!["security.manage".to_string(), "users.manage".to_string()];
    let names: Vec<_> = tools_for_capabilities(&admin)
        .iter()
        .map(|t| t.name)
        .collect();
    // users.manage is an umbrella for user.* tools.
    assert!(names.contains(&"users.create"));
    assert!(names.contains(&"users.token_create"));
    assert!(names.contains(&"permissions.group_create"));
    assert!(!names.contains(&"entities.list"));
}

#[test]
fn prepare_call_substitutes_path_and_splits_args() {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == "users.token_revoke")
        .unwrap();
    let out = prepare_call(
        "http://api:8010/api/v1",
        tool,
        &json!({ "id": "u-1", "token_id": "t-9" }),
    )
    .unwrap();
    assert_eq!(out.url, "http://api:8010/api/v1/users/u-1/tokens/t-9");
    assert!(out.query.is_empty());
}

#[test]
fn prepare_call_get_sends_query() {
    let tool = TOOLS.iter().find(|t| t.name == "entities.list").unwrap();
    let out = prepare_call(
        "http://api/api/v1",
        tool,
        &json!({ "type": "legal_entity", "limit": 5 }),
    )
    .unwrap();
    assert!(out.query.contains(&("type".into(), "legal_entity".into())));
    assert!(out.query.contains(&("limit".into(), "5".into())));
}

#[test]
fn prepare_call_post_sends_body() {
    let tool = TOOLS.iter().find(|t| t.name == "users.create").unwrap();
    let out = prepare_call(
        "http://api/api/v1",
        tool,
        &json!({
            "login": "svc-bot", "display_name": "Svc Bot",
            "is_service_account": true, "groups": ["viewer"],
        }),
    )
    .unwrap();
    assert_eq!(out.body["login"], "svc-bot");
    assert_eq!(out.body["is_service_account"], true);
}

#[test]
fn prepare_call_rejects_missing_required() {
    let tool = TOOLS.iter().find(|t| t.name == "users.create").unwrap();
    let err = prepare_call("http://a", tool, &json!({ "login": "x" })).unwrap_err();
    assert!(err.contains("display_name"), "{err}");
}

#[test]
fn prepare_call_rejects_unknown_arg() {
    let tool = TOOLS.iter().find(|t| t.name == "entities.get").unwrap();
    let err = prepare_call("http://a", tool, &json!({ "id": "x", "bogus": 1 })).unwrap_err();
    assert!(err.contains("bogus"), "{err}");
}

#[test]
fn prepare_call_rejects_unresolved_placeholder() {
    let tool = TOOLS.iter().find(|t| t.name == "entities.get").unwrap();
    let err = prepare_call("http://a", tool, &json!({})).unwrap_err();
    assert!(err.contains("id"), "{err}");
}

#[test]
fn prepare_call_path_arg_types() {
    // A numeric path arg used to collapse to "" and silently reroute
    // `entities.get {id}` onto the list endpoint; now numbers stringify
    // (needed for integer path params like chunk index), structured
    // values are still rejected.
    let tool = TOOLS.iter().find(|t| t.name == "entities.get").unwrap();
    let err = prepare_call("http://a", tool, &json!({ "id": [1, 2] })).unwrap_err();
    assert!(err.contains("must be a string"), "{err}");
    let err = prepare_call("http://a", tool, &json!({ "id": true })).unwrap_err();
    assert!(err.contains("must be a string"), "{err}");
    // Numbers become their canonical string form.
    let ok = prepare_call("http://a", tool, &json!({ "id": 42 })).unwrap();
    assert!(ok.url.ends_with("/entities/42"), "{}", ok.url);
}

#[test]
fn prepare_call_rejects_path_traversal() {
    let tool = TOOLS.iter().find(|t| t.name == "entities.get").unwrap();
    for bad in ["../users", "a/b", "x?y", "x#y", "", ".."] {
        let err = prepare_call("http://a", tool, &json!({ "id": bad })).unwrap_err();
        assert!(err.contains("invalid path argument"), "{bad} → {err}");
    }
    // A clean uuid still works.
    prepare_call(
        "http://a",
        tool,
        &json!({ "id": "00000000-0000-7000-9000-000000000001" }),
    )
    .unwrap();
}

// --- JSON-RPC layer ---------------------------------------------------------

fn ctx(caps: &[&str]) -> serde_json::Value {
    json!({
        "principal_id": "p1",
        "capabilities": caps,
    })
}

#[tokio::test]
async fn rpc_initialize() {
    let state = McpState::new("http://127.0.0.1:1/api/v1");
    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&[]),
        &json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}),
    )
    .await
    .unwrap();
    assert_eq!(res["result"]["protocolVersion"], "2025-06-18");
    assert!(res["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn rpc_notification_returns_none() {
    let state = McpState::new("http://127.0.0.1:1/api/v1");
    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&[]),
        &json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
    )
    .await;
    assert!(res.is_none());
}

#[tokio::test]
async fn rpc_tools_list_respects_capabilities() {
    let state = McpState::new("http://127.0.0.1:1/api/v1");
    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&["entity.read"]),
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
    )
    .await
    .unwrap();
    let names: Vec<&str> = res["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"entities.list"));
    assert!(!names.contains(&"users.create"));
}

#[tokio::test]
async fn rpc_tools_call_blocked_without_capability() {
    let state = McpState::new("http://127.0.0.1:1/api/v1");
    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&["entity.read"]),
        &json!({"jsonrpc":"2.0","id":3,"method":"tools/call",
                "params":{"name":"users.create","arguments":{"login":"x","display_name":"y"}}}),
    )
    .await
    .unwrap();
    assert_eq!(res["error"]["code"], -32602);
    assert!(res["error"]["message"]
        .as_str()
        .unwrap()
        .contains("user.create"));
}

#[tokio::test]
async fn rpc_unknown_method_and_unknown_tool() {
    let state = McpState::new("http://127.0.0.1:1/api/v1");
    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&[]),
        &json!({"jsonrpc":"2.0","id":4,"method":"no/such"}),
    )
    .await
    .unwrap();
    assert_eq!(res["error"]["code"], -32601);

    let res = handle_rpc(
        &state,
        "egr_x",
        &ctx(&[]),
        &json!({"jsonrpc":"2.0","id":5,"method":"tools/call",
                "params":{"name":"nope","arguments":{}}}),
    )
    .await
    .unwrap();
    assert_eq!(res["error"]["code"], -32602);
}

// --- API contract drift guards ------------------------------------------------

/// Param names must match the keys the EngineR API actually reads — a
/// drifted name silently does nothing or 422s.
#[test]
fn tool_params_match_api_contract_names() {
    let find = |n: &str| TOOLS.iter().find(|t| t.name == n).unwrap();
    let has = |n: &str, p: &str| find(n).params.iter().any(|x| x.name == p);
    let lacks = |n: &str, p: &str| !find(n).params.iter().any(|x| x.name == p);

    assert!(has("entities.create", "type_id"));
    assert!(lacks("entities.create", "type_alias"));
    assert!(has("entities.merge", "loser_public_id"));
    assert!(lacks("entities.merge", "source_id"));
    assert!(has("entities.graph", "max_depth"));
    assert!(lacks("entities.graph", "depth"));
    assert!(has("relations.create", "target_entity_id"));
    assert!(has("tasks.create", "due_date"));
    assert!(lacks("tasks.create", "due_at"));
    assert!(has("checks.create", "request_id"));
    assert!(has("schema.type_get", "alias"));
    assert!(has("schema.type_view", "alias"));
    assert!(has("schema.draft_update", "display_name"));
    assert!(lacks("schema.draft_update", "payload"));
    assert!(has("schema.draft_publish", "reason"));
    assert!(has("requests.create", "target_entity_id"));
    assert!(has("workflows.create", "definition_id"));
    assert!(has("workflows.transition", "to"));
    assert!(has("reports.generate", "entity_id"));
    assert!(has("connectors.create", "connector_type_id"));
    assert!(has("permissions.group_create", "principals"));
    assert!(lacks("permissions.group_create", "members"));
    assert!(has("permissions.group_update", "principals"));
    assert!(has("users.token_revoke", "token_id"));
}

#[test]
fn prepare_call_forwards_catalog_on_post_and_get() {
    // `catalog` is a Query param even on POST tools — it must land in
    // the query string, not the JSON body, or the API ignores it and
    // the call silently operates on the wrong catalog scope.
    let create = TOOLS.iter().find(|t| t.name == "entities.create").unwrap();
    let out = prepare_call(
        "http://a/api/v1",
        create,
        &json!({ "type_id": "legal_entity", "catalog": "wh", "fields": {} }),
    )
    .unwrap();
    assert!(
        out.query.contains(&("catalog".into(), "wh".into())),
        "catalog must be a query param: {:?}",
        out.query
    );
    assert!(
        out.body.get("catalog").is_none(),
        "catalog must not leak into body"
    );

    let audit = TOOLS.iter().find(|t| t.name == "audit.query").unwrap();
    let out = prepare_call(
        "http://a/api/v1",
        audit,
        &json!({ "catalog": "qa_t", "limit": 5 }),
    )
    .unwrap();
    assert!(out.query.contains(&("catalog".into(), "qa_t".into())));
}

#[test]
fn prepare_call_restore_sends_confirm_in_body() {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == "admin.dump_restore")
        .unwrap();
    let out = prepare_call(
        "http://a/api/v1",
        tool,
        &json!({ "id": "dump-1", "confirm": "RESTORE" }),
    )
    .unwrap();
    assert_eq!(out.url, "http://a/api/v1/admin/dump/dump-1/restore");
    assert_eq!(out.body["confirm"], "RESTORE");
    // Missing/wrong confirm must fail fast at the API — but a missing
    // argument is caught earlier by required-param validation.
    let err = prepare_call("http://a", tool, &json!({ "id": "d" })).unwrap_err();
    assert!(err.contains("confirm"), "{err}");
}

#[test]
fn dump_tools_require_security_manage() {
    let viewer = vec!["entity.read".to_string()];
    let names: Vec<_> = tools_for_capabilities(&viewer)
        .iter()
        .map(|t| t.name)
        .collect();
    for t in [
        "admin.dump_run",
        "admin.dump_list",
        "admin.dump_delete",
        "admin.dump_restore",
    ] {
        assert!(
            !names.contains(&t),
            "{t} must not be visible to entity.read"
        );
    }
    let admin = vec!["security.manage".to_string()];
    let names: Vec<_> = tools_for_capabilities(&admin)
        .iter()
        .map(|t| t.name)
        .collect();
    for t in [
        "admin.dump_run",
        "admin.dump_list",
        "admin.dump_delete",
        "admin.dump_restore",
    ] {
        assert!(names.contains(&t), "{t} must be visible to security.manage");
    }
}

#[test]
fn catalog_tools_present_and_scoped() {
    let find = |n: &str| TOOLS.iter().find(|t| t.name == n);
    for n in ["catalogs.list", "catalogs.get", "catalogs.create"] {
        assert!(find(n).is_some(), "missing tool {n}");
    }
    // Catalog-aware tools declare the `catalog` query param.
    for n in [
        "entities.list",
        "entities.create",
        "entities.update",
        "entities.query",
        "audit.query",
    ] {
        let t = find(n).unwrap_or_else(|| panic!("missing tool {n}"));
        assert!(
            t.params.iter().any(|p| p.name == "catalog"),
            "{n} missing catalog param"
        );
    }
    // Entity-detail tools keyed by uuid don't take a catalog selector
    // — the selector is a listing filter, not a security scope.
    for n in ["entities.retire", "entities.merge"] {
        let t = find(n).unwrap_or_else(|| panic!("missing tool {n}"));
        assert!(
            !t.params.iter().any(|p| p.name == "catalog"),
            "{n} should not take a catalog selector"
        );
    }
}

#[test]
fn prepare_call_decodes_binary_body() {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == "files.upload_chunk")
        .unwrap();
    let ok = prepare_call(
        "http://a",
        tool,
        &json!({ "id": "u1", "index": 0, "data_base64": "aGVsbG8=" }),
    )
    .unwrap();
    assert!(ok.url.ends_with("/files/upload/u1/chunks/0"), "{}", ok.url);
    assert_eq!(ok.raw_body, Some(b"hello".to_vec()));

    let err = prepare_call(
        "http://a",
        tool,
        &json!({ "id": "u1", "index": 0, "data_base64": "!!!" }),
    )
    .unwrap_err();
    assert!(err.contains("base64"), "{err}");
}

#[test]
fn prepare_call_lifts_if_match_header() {
    let tool = TOOLS
        .iter()
        .find(|t| t.name == "schema.draft_publish")
        .unwrap();
    let ok = prepare_call(
        "http://a",
        tool,
        &json!({ "id": "r1", "reason": "x", "if_match": "\"schema:27:1\"" }),
    )
    .unwrap();
    assert_eq!(ok.if_match.as_deref(), Some("\"schema:27:1\""));
    assert!(ok.body.get("if_match").is_none());
}

#[test]
fn embedded_agent_tool_doc_is_the_real_skill_md() {
    // The GET /agent-tool.md route serves this constant — make sure the
    // include_str! path resolves to the actual skill document.
    use enginer_mcp::AGENT_TOOL_MD;
    assert!(AGENT_TOOL_MD.len() > 2000, "skill doc suspiciously small");
    assert!(AGENT_TOOL_MD.starts_with("---\nname: enginer-mcp"));
    for marker in [
        "## 1. Authentication",
        "POST /mcp",
        "/agent-tool.md",
        "relation_groups",
    ] {
        assert!(AGENT_TOOL_MD.contains(marker), "missing marker: {marker}");
    }
}
