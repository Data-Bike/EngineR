//! Live-DB scenario tests for the PostgreSQL-backed entity layer.
//!
//! These tests exercise the real HTTP surface (auth, capability checks,
//! RLS transactions, identity-key dedup, schema lifecycle) against a running
//! PostgreSQL with the demo dataset seeded.
//!
//! Run locally:
//! ```sh
//! ENGINER_TEST_DATABASE_URL=postgres://enginer_app:enginer_app@localhost:6543/enginer \
//!   cargo test -p enginer-api --test pg_scenarios -- --test-threads=1
//! ```
//!
//! Without the env var every test is skipped, so the default `cargo test`
//! stays hermetic.

use enginer_api::{rocket_build, ApiConfig, AppState};
use rocket::http::{ContentType, Header, Method, Status};
use rocket::local::blocking::Client;
use serde_json::{json, Value};
use std::sync::Arc;

struct Harness {
    client: Client,
    csrf: String,
}

fn state() -> Option<AppState> {
    let url = std::env::var("ENGINER_TEST_DATABASE_URL").ok()?;
    let exec: Arc<enginer_db_runtime::AsyncStdDbExecutor> = Arc::new(
        enginer_db_runtime::AsyncStdDbExecutor::spawn(enginer_db_runtime::DbExecutorConfig {
            database_url: url,
            max_connections: 4,
            acquire_timeout: std::time::Duration::from_secs(10),
            queue_capacity: 64,
        })
        .ok()?,
    );
    let security_client: Option<Arc<dyn enginer_security_api::client::SecurityClient>> = Some(
        enginer_security_api::local_client(Arc::new(TestSnapshotProvider { exec: exec.clone() })),
    );
    Some(AppState::new(
        ApiConfig::default(),
        Some(exec),
        security_client,
    ))
}

/// Thread-bridge snapshot provider (same pattern as `main.rs`).
struct TestSnapshotProvider {
    exec: Arc<enginer_db_runtime::AsyncStdDbExecutor>,
}

impl enginer_security_api::client::SnapshotProvider for TestSnapshotProvider {
    fn snapshot(
        &self,
        tenant: enginer_domain::TenantId,
        principal: enginer_domain::PrincipalId,
        policy_revision: Option<i64>,
    ) -> Result<enginer_security_pdp::PdpSnapshot, enginer_security_api::client::SecurityClientError>
    {
        use enginer_security_api::client::SecurityClientError as Err_;
        let exec = self.exec.clone();
        let joined = std::thread::scope(|scope| {
            scope
                .spawn(move || {
                    async_std::task::block_on(exec.run(
                        enginer_db_runtime::Deadline::in_secs(10),
                        move |pool| async move {
                            let provider = enginer_persistence::PgSecuritySnapshotProvider::new(
                                Arc::new(pool),
                            );
                            match provider.snapshot(tenant, principal, policy_revision) {
                                Ok(snap) => Ok(serde_json::json!({
                                    "policy_revision": snap.policy_revision,
                                    "memberships": snap.memberships,
                                    "rules": snap.rules,
                                })),
                                Err(e) => Err(enginer_db_runtime::DbError::Other {
                                    message: e.to_string(),
                                }),
                            }
                        },
                    ))
                })
                .join()
        });
        let v = match joined {
            Ok(Ok(v)) => v,
            Ok(Err(e)) => {
                return Err(Err_::Unavailable {
                    message: e.to_string(),
                })
            }
            Err(_) => {
                return Err(Err_::Unavailable {
                    message: "snapshot thread panicked".to_string(),
                })
            }
        };
        let memberships =
            serde_json::from_value(v["memberships"].clone()).map_err(|e| Err_::Unavailable {
                message: format!("memberships decode: {e}"),
            })?;
        let rules = serde_json::from_value(v["rules"].clone()).map_err(|e| Err_::Unavailable {
            message: format!("rules decode: {e}"),
        })?;
        Ok(enginer_security_pdp::PdpSnapshot {
            policy_revision: v["policy_revision"].as_i64().unwrap_or_default(),
            memberships,
            rules,
        })
    }
}

/// Fresh harness per test: the blocking Rocket client is not `Sync`, so it
/// can't be shared through a static.
fn harness() -> Option<Harness> {
    let state = state()?;
    let client = Client::tracked(rocket_build(state)).expect("rocket client");
    let login = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "admin", "password": "enginer-demo"}).to_string())
        .dispatch();
    if login.status() != Status::Ok {
        eprintln!(
            "pg_scenarios: login failed ({}): seed demo data not loaded?",
            login.status()
        );
        return None;
    }
    let body = login.into_json::<Value>().unwrap_or_default();
    let csrf = body["data"]["csrf_token"].as_str()?.to_string();
    Some(Harness { client, csrf })
}

fn req(h: &Harness, method: Method, path: &str, body: Option<Value>) -> (Status, Value) {
    let mut r = h
        .client
        .req(method, path)
        .header(Header::new("X-CSRF-Token", h.csrf.clone()));
    if let Some(b) = body {
        r = r.header(ContentType::JSON).body(b.to_string());
    }
    let resp = r.dispatch();
    let status = resp.status();
    let payload = resp.into_json().unwrap_or(Value::Null);
    (status, payload)
}

fn create_entity(h: &Harness, inn: &str, name: &str, extra: Vec<Value>) -> (Status, Value) {
    let mut statements = vec![
        json!({"predicate_id": "name", "value": {"kind": "text", "value": name}}),
        json!({"predicate_id": "inn", "value": {"kind": "text", "value": inn}}),
    ];
    statements.extend(extra);
    req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": "legal_entity",
            "identity_mode": "return_existing",
            "statements": statements,
        })),
    )
}

fn query_by_inn(h: &Harness, inn: &str) -> (Status, Value) {
    req(
        &h,
        Method::Post,
        "/api/v1/entities/query",
        Some(json!({
            "from": {"type_id": "legal_entity", "alias": "e"},
            "select": [
                {"field": {"system": "public_id"}, "alias": "public_id"},
                {"field": {"system": "display_name"}, "alias": "display_name"}
            ],
            "where": {"comparison": {"field": {"predicate": "inn"}, "op": "eq", "value": inn}},
            "page": {"size": 10}
        })),
    )
}

fn uniq(prefix: &str) -> String {
    // v4, not v7: the first 8 hex chars of a v7 uuid are the timestamp, which
    // is identical for uuids minted in the same test run.
    format!(
        "{prefix}{}",
        uuid::Uuid::new_v4().simple().to_string()[..8].to_uppercase()
    )
}

// ---------------------------------------------------------------------------
// Scenarios
// ---------------------------------------------------------------------------

#[test]
fn login_returns_session_and_context() {
    let Some(h) = harness() else { return };
    let (status, body) = req(&h, Method::Get, "/api/v1/me/context", None);
    assert_eq!(status, Status::Ok);
    assert!(body["data"]["principal_id"].is_string() || body["data"]["principal"].is_object());
}

#[test]
fn create_entity_then_appears_in_list_and_query() {
    let Some(h) = harness() else { return };
    let inn = uniq("77");
    let (status, body) = create_entity(&h, &inn, "ООО «Сценарий-1»", vec![]);
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["outcome"], "created");
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    // Card is readable.
    let (status, body) = req(&h, Method::Get, &format!("/api/v1/entities/{pid}"), None);
    assert_eq!(status, Status::Ok);
    assert_eq!(body["data"]["display_name"], "ООО «Сценарий-1»");
    assert_eq!(body["data"]["fields"]["inn"], inn);

    // Query with a `where` clause finds exactly this entity — the clause is
    // honoured instead of being silently ignored.
    let (status, body) = query_by_inn(&h, &inn);
    assert_eq!(status, Status::Ok);
    let rows = body["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1, "expected one match for inn={inn}: {body}");
    assert_eq!(rows[0]["public_id"], pid);
}

#[test]
fn where_clause_does_not_return_unrelated_entities() {
    let Some(h) = harness() else { return };
    // Regression: the `where` AST used to be ignored, so the dedup check
    // flagged every entity of the type as a duplicate.
    let (status, body) = query_by_inn(&h, "0000000000-nosuch");
    assert_eq!(status, Status::Ok);
    let rows = body["data"]["rows"].as_array().unwrap();
    assert!(rows.is_empty(), "unexpected matches: {body}");
}

#[test]
fn empty_optional_identity_field_does_not_dedup() {
    let Some(h) = harness() else { return };
    // Two distinct entities sharing an EMPTY optional identity field (ogrn)
    // must not collapse into each other.
    let inn_a = uniq("55");
    let inn_b = uniq("55");
    let empty_ogrn = vec![json!({"predicate_id": "ogrn", "value": {"kind": "text", "value": ""}})];
    let (status, body) = create_entity(&h, &inn_a, "ООО «Первая»", empty_ogrn.clone());
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["outcome"], "created");

    let (status, body) = create_entity(&h, &inn_b, "ООО «Вторая»", empty_ogrn);
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body["data"]["outcome"], "created",
        "empty identity component produced a false dedup: {body}"
    );
}

#[test]
fn same_identity_value_deduplicates() {
    let Some(h) = harness() else { return };
    let inn = uniq("66");
    let (status, body) = create_entity(&h, &inn, "ООО «Оригинал»", vec![]);
    assert_eq!(status, Status::Ok);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    let (status, body) = create_entity(&h, &inn, "ООО «Копия»", vec![]);
    assert_eq!(status, Status::Ok);
    assert_eq!(body["data"]["outcome"], "deduplicated");
    assert_eq!(body["data"]["public_id"], pid);
}

#[test]
fn update_requires_current_revision() {
    let Some(h) = harness() else { return };
    let inn = uniq("78");
    let (_, body) = create_entity(&h, &inn, "ООО «Ревизия»", vec![]);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    // Stale revision is rejected.
    let (status, _) = req(
        &h,
        Method::Put,
        &format!("/api/v1/entities/{pid}"),
        Some(json!({
            "revision": 99,
            "fields": [{"predicate_id": "address", "value": {"kind": "text", "value": "x"}}]
        })),
    );
    assert_eq!(status, Status::Conflict);

    // Current revision succeeds and is visible on reload.
    let (status, body) = req(
        &h,
        Method::Put,
        &format!("/api/v1/entities/{pid}"),
        Some(json!({
            "revision": 1,
            "fields": [{"predicate_id": "address", "value": {"kind": "text", "value": "ул. Проверочная, 7"}}]
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["fields"]["address"], "ул. Проверочная, 7");
    assert_eq!(body["data"]["revision"], 2);
}

#[test]
fn update_recomputes_identity_keys() {
    let Some(h) = harness() else { return };
    let inn_a = uniq("79");
    let inn_b = uniq("79");
    let (_, body) = create_entity(&h, &inn_a, "ООО «Ключи-А»", vec![]);
    let pid_a = body["data"]["public_id"].as_str().unwrap().to_string();
    let (_, body) = create_entity(&h, &inn_b, "ООО «Ключи-Б»", vec![]);
    let pid_b = body["data"]["public_id"].as_str().unwrap().to_string();

    // Re-pointing B's INN onto A's value is an identity collision → 409.
    let (status, _) = req(
        &h,
        Method::Put,
        &format!("/api/v1/entities/{pid_b}"),
        Some(json!({
            "revision": 1,
            "fields": [{"predicate_id": "inn", "value": {"kind": "text", "value": inn_a}}]
        })),
    );
    assert_eq!(status, Status::Conflict);

    // Unique new INN: accepted, and the OLD key no longer dedups to B.
    let inn_c = uniq("79");
    let (status, _) = req(
        &h,
        Method::Put,
        &format!("/api/v1/entities/{pid_b}"),
        Some(json!({
            "revision": 1,
            "fields": [{"predicate_id": "inn", "value": {"kind": "text", "value": inn_c.clone()}}]
        })),
    );
    assert_eq!(status, Status::Ok);

    let (status, body) = create_entity(&h, &inn_b, "ООО «Наследник-Б»", vec![]);
    assert_eq!(status, Status::Ok);
    assert_eq!(
        body["data"]["outcome"], "created",
        "stale identity key kept matching after update: {body}"
    );

    let (status, body) = create_entity(&h, &inn_c, "ООО «Ещё-Копия»", vec![]);
    assert_eq!(status, Status::Ok);
    assert_eq!(body["data"]["outcome"], "deduplicated");
    assert_eq!(body["data"]["public_id"], pid_b);
    let _ = pid_a;
}

#[test]
fn relation_create_and_graph_traversal() {
    let Some(h) = harness() else { return };
    let inn_a = uniq("81");
    let inn_b = uniq("82");
    let (_, body) = create_entity(&h, &inn_a, "ООО «Связь-А»", vec![]);
    let pid_a = body["data"]["public_id"].as_str().unwrap().to_string();
    let (_, body) = create_entity(&h, &inn_b, "ООО «Связь-Б»", vec![]);
    let pid_b = body["data"]["public_id"].as_str().unwrap().to_string();

    let (status, body) = req(
        &h,
        Method::Post,
        &format!("/api/v1/entities/{pid_a}/relations"),
        Some(json!({"predicate": "owns", "target_entity_id": pid_b})),
    );
    assert_eq!(status, Status::Ok, "{body}");

    // Duplicate relation is rejected.
    let (status, _) = req(
        &h,
        Method::Post,
        &format!("/api/v1/entities/{pid_a}/relations"),
        Some(json!({"predicate": "owns", "target_entity_id": pid_b})),
    );
    assert_eq!(status, Status::Conflict);

    // Graph traversal from A reaches B in one hop.
    let (status, body) = req(
        &h,
        Method::Post,
        &format!("/api/v1/entities/{pid_a}/graph"),
        Some(json!({"max_depth": 1, "direction": "outgoing"})),
    );
    assert_eq!(status, Status::Ok);
    let node_ids: Vec<&str> = body["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|n| n["id"].as_str())
        .collect();
    assert!(node_ids.contains(&pid_b.as_str()), "{body}");
    let edge_labels: Vec<&str> = body["data"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|e| e["label"].as_str())
        .collect();
    assert!(
        edge_labels.iter().any(|l| *l == "Владеет"),
        "{edge_labels:?}"
    );
}

fn create_typed(h: &Harness, type_id: &str, statements: Vec<Value>) -> (Status, Value) {
    req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": type_id,
            "identity_mode": "return_existing",
            "statements": statements,
        })),
    )
}

#[test]
fn relations_include_group_preview_and_trans_through() {
    let Some(h) = harness() else { return };
    // ФЛ → holds_position → Должность(trans) → {ЮЛ, рабочий телефон}.
    let (_, b) = create_entity(&h, &uniq("83"), "ООО «Цель»", vec![]);
    let org = b["data"]["public_id"].as_str().unwrap().to_string();
    let (st, b) = create_typed(
        &h,
        "position",
        vec![json!({"predicate_id": "title", "value": {"kind": "text", "value": "Директор"}})],
    );
    assert_eq!(st, Status::Ok, "{b}");
    let pos = b["data"]["public_id"].as_str().unwrap().to_string();
    let (st, b) = create_typed(
        &h,
        "contact_point",
        vec![
            json!({"predicate_id": "cp_kind", "value": {"kind": "text", "value": "phone"}}),
            json!({"predicate_id": "cp_number", "value": {"kind": "text", "value": "123-45-67"}}),
        ],
    );
    assert_eq!(st, Status::Ok, "{b}");
    let cp = b["data"]["public_id"].as_str().unwrap().to_string();
    let (st, b) = create_typed(
        &h,
        "person",
        vec![
            // full_name is an identity field — a fixed name would dedup to a
            // previous run's person (with someone else's relations).
            json!({"predicate_id": "full_name", "value": {"kind": "text", "value": format!("Сидоров {}", uniq("sid"))}}),
            json!({"predicate_id": "inn", "value": {"kind": "text", "value": uniq("84")}}),
        ],
    );
    assert_eq!(st, Status::Ok, "{b}");
    let person = b["data"]["public_id"].as_str().unwrap().to_string();

    for (src, pred, tgt) in [
        (&person, "holds_position", &pos),
        (&pos, "position_at", &org),
        (&pos, "position_work_phone", &cp),
    ] {
        let (st, b) = req(
            &h,
            Method::Post,
            &format!("/api/v1/entities/{src}/relations"),
            Some(json!({"predicate": pred, "target_entity_id": tgt})),
        );
        assert_eq!(st, Status::Ok, "{pred}: {b}");
    }

    let (st, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/entities/{person}/relations"),
        None,
    );
    assert_eq!(st, Status::Ok);
    let rels = body["data"].as_array().unwrap();
    let holds = rels
        .iter()
        .find(|r| r["predicate"] == "holds_position")
        .unwrap_or_else(|| panic!("holds_position missing: {body}"));
    assert_eq!(holds["group"], "employment", "{holds}");
    assert_eq!(holds["trans"], true, "{holds}");
    assert!(
        holds["preview"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["field"] == "title" && f["value"] == "Директор"),
        "{holds}"
    );
    let through = holds["through"].as_array().unwrap();
    let through_targets: Vec<&str> = through
        .iter()
        .filter_map(|e| e["target_entity_id"].as_str())
        .collect();
    assert!(through_targets.contains(&org.as_str()), "{through:?}");
    assert!(through_targets.contains(&cp.as_str()), "{through:?}");
    // A plain relation stays un-grouped-by-trans but still carries its group.
    let (st, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/entities/{org}/relations"),
        None,
    );
    assert_eq!(st, Status::Ok);
    let incoming = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["predicate"] == "position_at")
        .unwrap_or_else(|| panic!("position_at missing: {body}"));
    assert_eq!(incoming["direction"], "incoming");
    assert_eq!(incoming["group"], "employment");
    assert_eq!(incoming["trans"], true); // the position is still a trans-object
}

#[test]
fn relations_to_same_target_keep_fields_on_every_record() {
    let Some(h) = harness() else { return };
    // Two relations from the person to the SAME company: the second record
    // must still carry display_name + preview (fields_map reuse regression).
    let (_, b) = create_entity(&h, &uniq("85"), "ООО «Дубль»", vec![]);
    let org = b["data"]["public_id"].as_str().unwrap().to_string();
    let (st, b) = create_typed(
        &h,
        "person",
        vec![
            json!({"predicate_id": "full_name", "value": {"kind": "text", "value": format!("Кузнецов {}", uniq("kuz"))}}),
            json!({"predicate_id": "inn", "value": {"kind": "text", "value": uniq("86")}}),
        ],
    );
    assert_eq!(st, Status::Ok, "{b}");
    let person = b["data"]["public_id"].as_str().unwrap().to_string();

    for pred in ["works_for", "owns"] {
        let (st, b) = req(
            &h,
            Method::Post,
            &format!("/api/v1/entities/{person}/relations"),
            Some(json!({"predicate": pred, "target_entity_id": org})),
        );
        assert_eq!(st, Status::Ok, "{pred}: {b}");
    }

    let (st, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/entities/{person}/relations"),
        None,
    );
    assert_eq!(st, Status::Ok);
    let rels = body["data"].as_array().unwrap();
    let to_org: Vec<&Value> = rels
        .iter()
        .filter(|r| r["target_entity_id"] == org)
        .collect();
    assert_eq!(to_org.len(), 2, "{body}");
    for r in to_org {
        assert_eq!(
            r["target_display_name"], "ООО «Дубль»",
            "second relation to same target lost its display name: {r}"
        );
        assert!(
            r["preview"]
                .as_array()
                .map(|a| !a.is_empty())
                .unwrap_or(false),
            "second relation to same target lost its preview: {r}"
        );
    }
}

#[test]
fn schema_draft_type_publish_and_entity_create() {
    let Some(h) = harness() else { return };
    let alias = uniq("test_type_").to_lowercase();

    // 1. Draft from the published base.
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/schemas/drafts",
        Some(json!({"name": "test draft", "description": "pg_scenarios"})),
    );
    assert_eq!(status, Status::Created, "{body}");
    let draft_id = body["data"]["revision_id"].as_str().unwrap().to_string();
    let etag = body["data"]["etag"].as_str().unwrap().to_string();
    let base_types = body["data"]["types"].clone();
    let base_predicates = body["data"]["predicates"].clone();
    // Publish requires the draft's semantic_version to exceed every published
    // revision — reuse the auto-assigned version (already bumped past latest).
    let new_version = body["data"]["semantic_version"]
        .as_str()
        .unwrap_or("0.0.1")
        .to_string();

    // 2. Add a new type entry to the draft (with an identity field — publish
    // must derive an identity policy for it).
    let mut types = base_types.as_array().cloned().unwrap_or_default();
    types.push(json!({
        "type_id": alias,
        "stable_alias": alias,
        "entity_class": "data",
        "label": "Тестовый тип",
        "fields": [
            {"alias": "name", "label": "Наименование", "kind": "text", "required": true},
            {"alias": "test_code", "label": "Код", "kind": "text", "identity": true}
        ]
    }));
    let mut predicates = base_predicates.as_array().cloned().unwrap_or_default();
    if !predicates
        .iter()
        .any(|p| p.get("stable_alias").and_then(|a| a.as_str()) == Some("test_code"))
    {
        predicates.push(json!({
            "stable_alias": "test_code", "scalar_kind": "text", "label": "Код",
            "participates_in_identity": true
        }));
    }
    let draft_path = format!("/api/v1/schemas/drafts/{draft_id}");
    let r = h
        .client
        .req(Method::Put, &draft_path)
        .header(Header::new("X-CSRF-Token", h.csrf.clone()))
        .header(Header::new("If-Match", etag))
        .header(ContentType::JSON)
        .body(
            json!({
                "display_name": "test draft",
                "description": "pg_scenarios",
                "semantic_version": new_version,
                "types": types,
                "predicates": predicates,
                "changes": []
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(r.status(), Status::Ok);

    // 2b. A metadata-only PUT must NOT wipe types/predicates (regression:
    // serde(default) Vec fields used to clear the whole schema content).
    let (status, body) = req(&h, Method::Get, &draft_path, None);
    assert_eq!(status, Status::Ok);
    let etag2 = body["data"]["etag"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    let r = h
        .client
        .req(Method::Put, &draft_path)
        .header(Header::new("X-CSRF-Token", h.csrf.clone()))
        .header(Header::new("If-Match", etag2))
        .header(ContentType::JSON)
        .body(
            json!({
                "display_name": "test draft renamed",
                "description": "pg_scenarios",
                "semantic_version": new_version
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(r.status(), Status::Ok);
    let (status, body) = req(&h, Method::Get, &draft_path, None);
    assert_eq!(status, Status::Ok);
    assert_eq!(
        body["data"]["types"].as_array().map(|t| t.len()),
        Some(types.len()),
        "metadata-only update wiped draft types"
    );

    // 3. Validate → impact → publish.
    for step in ["validate", "impact"] {
        let (status, body) = req(
            &h,
            Method::Post,
            &format!("/api/v1/schemas/drafts/{draft_id}/{step}"),
            None,
        );
        assert_eq!(status, Status::Ok, "{step}: {body}");
    }
    // validate/impact each bump the draft version — fetch the fresh etag.
    let (status, body) = req(&h, Method::Get, &draft_path, None);
    assert_eq!(status, Status::Ok);
    let etag3 = body["data"]["etag"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let publish_path = format!("/api/v1/schemas/drafts/{draft_id}/publish");
    let publish = h
        .client
        .req(Method::Post, &publish_path)
        .header(Header::new("X-CSRF-Token", h.csrf.clone()))
        .header(Header::new("If-Match", etag3))
        .header(ContentType::JSON)
        .body(json!({"reason": "pg_scenarios test publish"}).to_string())
        .dispatch();
    assert_eq!(publish.status(), Status::Ok);

    // 4. The new type is resolvable and accepts entities.
    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/schemas/types/{alias}"),
        None,
    );
    assert_eq!(status, Status::Ok, "{body}");

    // 4a. Required fields are enforced: missing `name` -> 422, blank -> 422.
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": alias,
            "statements": [{"predicate_id": "test_code", "value": {"kind": "text", "value": "X-NONAME"}}]
        })),
    );
    assert_eq!(
        status,
        Status::UnprocessableEntity,
        "missing required: {body}"
    );
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": alias,
            "fields": {"name": "   "}
        })),
    );
    assert_eq!(
        status,
        Status::UnprocessableEntity,
        "blank required: {body}"
    );

    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": alias,
            "identity_mode": "return_existing",
            "statements": [{"predicate_id": "name", "value": {"kind": "text", "value": "Тест-объект"}}]
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["outcome"], "created");

    // 5. Identity policy was derived at publish: the identity field dedups.
    let code = format!("CODE-{alias}");
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": alias,
            "statements": [
                {"predicate_id": "name", "value": {"kind": "text", "value": "A"}},
                {"predicate_id": "test_code", "value": {"kind": "text", "value": code}}
            ]
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": alias,
            "statements": [
                {"predicate_id": "name", "value": {"kind": "text", "value": "B"}},
                {"predicate_id": "test_code", "value": {"kind": "text", "value": code}}
            ]
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(
        body["data"]["outcome"], "deduplicated",
        "identity policy not derived on publish: {body}"
    );
}

#[test]
fn unauthenticated_requests_are_rejected() {
    let Some(state) = state() else { return };
    let client = Client::tracked(rocket_build(state)).expect("rocket client");
    let resp = client.get("/api/v1/entities").dispatch();
    assert_eq!(resp.status(), Status::Unauthorized);
}

#[test]
fn persona_without_create_capability_is_denied() {
    let Some(state) = state() else { return };
    let client = Client::tracked(rocket_build(state)).expect("rocket client");
    // risk_analyst has read/query capabilities but no entity.create.
    let login = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": "risk_analyst", "password": "enginer-demo"}).to_string())
        .dispatch();
    assert_eq!(login.status(), Status::Ok);
    let body = login.into_json::<Value>().unwrap_or_default();
    let csrf = body["data"]["csrf_token"].as_str().unwrap().to_string();

    let resp = client
        .post("/api/v1/entities")
        .header(Header::new("X-CSRF-Token", csrf))
        .header(ContentType::JSON)
        .body(
            json!({
                "type_id": "legal_entity",
                "statements": [{"predicate_id": "name", "value": {"kind": "text", "value": "x"}}]
            })
            .to_string(),
        )
        .dispatch();
    assert_eq!(resp.status(), Status::Forbidden);
}

// ---------------------------------------------------------------------------
// Authorization scenarios: role tree, data rules, URL rules, user admin
// ---------------------------------------------------------------------------

/// Login as a demo persona or a user created via POST /users.
fn harness_as(login: &str, password: &str) -> Option<Harness> {
    let state = state()?;
    let client = Client::tracked(rocket_build(state)).expect("rocket client");
    let resp = client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": login, "password": password}).to_string())
        .dispatch();
    if resp.status() != Status::Ok {
        eprintln!("pg_scenarios: login {login} -> {}", resp.status());
        return None;
    }
    let body = resp.into_json::<Value>().unwrap_or_default();
    let csrf = body["data"]["csrf_token"].as_str()?.to_string();
    Some(Harness { client, csrf })
}

/// Create a permission group; returns its id.
fn create_group(
    h: &Harness,
    name: &str,
    class: &str,
    parent_id: Option<&str>,
    rules: Vec<Value>,
) -> (Status, Value) {
    req(
        h,
        Method::Post,
        "/api/v1/permissions/groups",
        Some(json!({
            "name": name, "class": class, "parent_id": parent_id,
            "principals": [], "rules": rules,
        })),
    )
}

/// Create a user belonging to the given group aliases/ids; returns (status, pid).
fn create_user(h: &Harness, login: &str, groups: Vec<&str>) -> (Status, String) {
    let (status, body) = req(
        h,
        Method::Post,
        "/api/v1/users",
        Some(json!({
            "login": login,
            "display_name": format!("Тест {login}"),
            "password": "test-pass-12345",
            "groups": groups,
        })),
    );
    let pid = body["data"]["principal_id"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    (status, pid)
}

fn delete_group(h: &Harness, id: &str) {
    let _ = req(
        h,
        Method::Delete,
        &format!("/api/v1/permissions/groups/{id}"),
        None,
    );
}

#[test]
fn users_api_denied_without_capability() {
    let Some(h) = harness() else { return };
    // A capability group granting only entity.read — no user.* access.
    let login = uniq("noauth_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("ent_readers_").to_lowercase(),
        "capability",
        None,
        vec![json!({"effect":"allow","resource_type":"capability","action":"entity.read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();

    let (status, _) = create_user(&h, &login, vec![&gid]);
    assert_eq!(status, Status::Ok, "create user");

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, _) = req(&u, Method::Get, "/api/v1/users", None);
    assert_eq!(
        status,
        Status::Forbidden,
        "user without user.read must not list users"
    );
    delete_group(&h, &gid);
}

#[test]
fn user_read_grants_list_but_not_create() {
    let Some(h) = harness() else { return };
    let login = uniq("viewer_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("user_viewers_").to_lowercase(),
        "capability",
        None,
        vec![json!({"effect":"allow","resource_type":"capability","action":"user.read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    create_user(&h, &login, vec![&gid]);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(&u, Method::Get, "/api/v1/users", None);
    assert_eq!(status, Status::Ok, "{body}");
    assert!(body["data"]
        .as_array()
        .map(|a| !a.is_empty())
        .unwrap_or(false));

    // But creation is denied — user.create is a separate capability.
    let (status, _) = create_user(&u, &uniq("sub_").to_lowercase(), vec![]);
    assert_eq!(status, Status::Forbidden, "user.create must be required");
    delete_group(&h, &gid);
}

#[test]
fn role_inheritance_and_cycle_prevention() {
    let Some(h) = harness() else { return };

    // Entity that will be denied through the parent role.
    let (_, body) = create_entity(&h, &uniq("90"), "ООО «Скрытый»", vec![]);
    let hidden_pid = body["data"]["public_id"].as_str().unwrap().to_string();

    // Parent data role denies read on that one entity.
    let (status, body) = create_group(
        &h,
        &uniq("deny_parent_").to_lowercase(),
        "data",
        None,
        vec![json!({"effect":"deny","resource_type":"entity","entity":hidden_pid,"action":"read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let parent_id = body["data"]["id"].as_str().unwrap().to_string();

    // Child role inherits the parent (no rules of its own).
    let (status, body) = create_group(
        &h,
        &uniq("deny_child_").to_lowercase(),
        "data",
        Some(&parent_id),
        vec![],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let child_id = body["data"]["id"].as_str().unwrap().to_string();

    let login = uniq("child_").to_lowercase();
    // Membership: capability entity.read via 'managers', data via child role.
    let (status, _) = create_user(&h, &login, vec!["managers", &child_id]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, _) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{hidden_pid}"),
        None,
    );
    assert_eq!(
        status,
        Status::NotFound,
        "inherited deny must hide the entity (1.3)"
    );

    // Role cycle prevention: making the parent a child of its own child fails.
    let (status, _) = req(
        &h,
        Method::Put,
        &format!("/api/v1/permissions/groups/{parent_id}"),
        Some(json!({
            "name": "x", "class": "data", "parent_id": child_id,
            "principals": [], "rules": [],
        })),
    );
    assert!(
        status != Status::Ok,
        "cycle must be rejected by the trigger, got {status}"
    );

    delete_group(&h, &child_id);
    delete_group(&h, &parent_id);
}

#[test]
fn url_rule_denies_api_path() {
    let Some(h) = harness() else { return };
    let login = uniq("norep_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("deny_reports_").to_lowercase(),
        "capability",
        None,
        vec![
            json!({"effect":"deny","resource_type":"url","path":"/api/v1/reports","action":"read"}),
        ],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let (status, _) = create_user(&h, &login, vec!["managers", &gid]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, _) = req(&u, Method::Get, "/api/v1/reports", None);
    assert_eq!(
        status,
        Status::Forbidden,
        "deny URL rule must block the API"
    );
    // Other routes unaffected.
    let (status, _) = req(&u, Method::Get, "/api/v1/entities", None);
    assert_eq!(status, Status::Ok);
    delete_group(&h, &gid);
}

#[test]
fn field_mask_redacts_value() {
    let Some(h) = harness() else { return };
    let inn = uniq("44");
    let (_, body) = create_entity(&h, &inn, "ООО «Маска»", vec![]);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    let login = uniq("masked_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("mask_inn_").to_lowercase(),
        "data",
        None,
        vec![json!({
            "effect":"allow","resource_type":"field",
            "type_alias":"legal_entity","field":"inn",
            "action":"read","mask":"partial-mask"
        })],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let (status, _) = create_user(&h, &login, vec!["managers", "all_data", &gid]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(&u, Method::Get, &format!("/api/v1/entities/{pid}"), None);
    assert_eq!(status, Status::Ok, "{body}");
    let masked = body["data"]["fields"]["inn"].as_str().unwrap_or_default();
    assert_ne!(masked, inn, "inn must be masked for this user");
    assert!(masked.contains("***"), "partial mask expected: {masked}");
    delete_group(&h, &gid);
}

#[test]
fn history_respects_field_mask() {
    // Regression: the history diff used to bypass field permissions — a
    // masked/denied field leaked its cleartext via old_value/new_value.
    let Some(h) = harness() else { return };
    let inn = uniq("44");
    let (_, body) = create_entity(&h, &inn, "ООО «История-маска»", vec![]);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    let login = uniq("histmask_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("hist_mask_").to_lowercase(),
        "data",
        None,
        vec![json!({
            "effect":"allow","resource_type":"field",
            "type_alias":"legal_entity","field":"inn",
            "action":"read","mask":"partial-mask"
        })],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let (status, _) = create_user(&h, &login, vec!["managers", "all_data", &gid]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{pid}/history"),
        None,
    );
    assert_eq!(status, Status::Ok, "{body}");
    // No cleartext inn anywhere in the diff — masked exactly like the entity
    // view (partial-mask keeps the outer characters).
    let blob = body.to_string();
    assert!(
        !blob.contains(&inn),
        "history diff must not leak the masked value: {blob}"
    );
    delete_group(&h, &gid);
}

#[test]
fn history_hides_denied_field() {
    // Same class of leak via an outright deny — the change must not appear.
    let Some(h) = harness() else { return };
    let inn = uniq("44");
    let (_, body) = create_entity(&h, &inn, "ООО «История-доступ»", vec![]);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    let login = uniq("histdeny_").to_lowercase();
    let (status, body) = create_group(
        &h,
        &uniq("hist_deny_").to_lowercase(),
        "data",
        None,
        vec![json!({
            "effect":"deny","resource_type":"field",
            "type_alias":"legal_entity","field":"inn","action":"read"
        })],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let (status, _) = create_user(&h, &login, vec!["managers", "all_data", &gid]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{pid}/history"),
        None,
    );
    assert_eq!(status, Status::Ok, "{body}");
    let blob = body.to_string();
    assert!(
        !blob.contains(&inn),
        "denied field value must not leak via history: {blob}"
    );
    // And the denied field's changes are dropped, not just nulled.
    let leaked = body["data"]
        .as_array()
        .map(|entries| {
            entries.iter().any(|e| {
                e["changes"].as_array().is_some_and(|cs| {
                    cs.iter()
                        .any(|c| c["alias"].as_str() == Some("inn"))
                })
            })
        })
        .unwrap_or(false);
    assert!(!leaked, "denied field must be absent from history: {body}");
    delete_group(&h, &gid);
}

#[test]
fn hidden_entity_hides_its_relations() {
    let Some(h) = harness() else { return };
    let (_, body) = create_entity(&h, &uniq("91"), "ООО «Видимый»", vec![]);
    let pid_a = body["data"]["public_id"].as_str().unwrap().to_string();
    let (_, body) = create_entity(&h, &uniq("92"), "ООО «Невидимый»", vec![]);
    let pid_b = body["data"]["public_id"].as_str().unwrap().to_string();
    let (status, _) = req(
        &h,
        Method::Post,
        &format!("/api/v1/entities/{pid_a}/relations"),
        Some(json!({"predicate": "owns", "target_entity_id": pid_b})),
    );
    assert_eq!(status, Status::Ok);

    let (status, body) = create_group(
        &h,
        &uniq("deny_b_").to_lowercase(),
        "data",
        None,
        vec![json!({"effect":"deny","resource_type":"entity","entity":pid_b,"action":"read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let login = uniq("hide_").to_lowercase();
    create_user(&h, &login, vec!["managers", "all_data", &gid]);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{pid_a}/relations"),
        None,
    );
    assert_eq!(status, Status::Ok);
    let targets: Vec<&str> = body["data"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| r["target_entity_id"].as_str())
                .collect()
        })
        .unwrap_or_default();
    assert!(
        !targets.contains(&pid_b.as_str()),
        "relation to hidden entity must not leak: {body}"
    );
    delete_group(&h, &gid);
}

#[test]
fn user_detail_activity_and_filters() {
    let Some(h) = harness() else { return };
    let login = uniq("act_").to_lowercase();
    let (status, pid) = create_user(&h, &login, vec!["managers", "all_data"]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let inn = uniq("93");
    let (status, body) = create_entity(&u, &inn, "ООО «Активность»", vec![]);
    assert_eq!(status, Status::Ok, "{body}");
    let entity_pid = body["data"]["public_id"].as_str().unwrap().to_string();
    let (status, _) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{entity_pid}"),
        None,
    );
    assert_eq!(status, Status::Ok);

    let (status, body) = req(&h, Method::Get, &format!("/api/v1/users?q={login}"), None);
    assert_eq!(status, Status::Ok);
    let found = body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["principal_id"].as_str() == Some(pid.as_str()));
    assert!(found, "user must be searchable by display_name: {body}");

    let (status, body) = req(&h, Method::Get, &format!("/api/v1/users/{pid}"), None);
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["login"], login);
    assert!(body["data"]["effective_roles"]
        .as_array()
        .map(|a| a.iter().any(|r| r.as_str() == Some("managers")))
        .unwrap_or(false));

    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users/{pid}/activity"),
        None,
    );
    assert_eq!(status, Status::Ok, "{body}");
    let created = body["data"]["created"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    assert!(
        created
            .iter()
            .any(|c| c["public_id"].as_str() == Some(entity_pid.as_str())),
        "created entity must appear in activity: {body}"
    );
    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users/{pid}/activity?q=entities"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(body["data"]["log"].is_array());
}

#[test]
fn role_filter_with_and_without_inheritance() {
    let Some(h) = harness() else { return };

    let (status, body) = create_group(&h, &uniq("parent_f_").to_lowercase(), "data", None, vec![]);
    assert_eq!(status, Status::Ok, "{body}");
    let parent_id = body["data"]["id"].as_str().unwrap().to_string();
    let (status, body) = create_group(
        &h,
        &uniq("child_f_").to_lowercase(),
        "data",
        Some(&parent_id),
        vec![],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let child_id = body["data"]["id"].as_str().unwrap().to_string();

    let login = uniq("inher_").to_lowercase();
    let (status, pid) = create_user(&h, &login, vec![&child_id]);
    assert_eq!(status, Status::Ok);

    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users?group_id={child_id}"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["principal_id"].as_str() == Some(pid.as_str())));

    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users?group_id={parent_id}"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(
        !body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["principal_id"].as_str() == Some(pid.as_str())),
        "without inheritance the parent role must not include child members"
    );

    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users?group_id={parent_id}&inherited=1"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x["principal_id"].as_str() == Some(pid.as_str())),
        "inherited filter must descend into child roles"
    );

    delete_group(&h, &child_id);
    delete_group(&h, &parent_id);
}

#[test]
fn superuser_sees_data_but_not_admin_api() {
    let Some(h) = harness() else { return };
    let login = uniq("su_").to_lowercase();
    let (status, _) = create_user(&h, &login, vec!["superuser"]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, _) = req(&u, Method::Get, "/api/v1/entities", None);
    assert_eq!(status, Status::Ok, "superuser must read data");
    let (status, _) = req(&u, Method::Get, "/api/v1/users", None);
    assert_eq!(status, Status::Forbidden, "superuser must not reach /users");
    let (status, _) = req(&u, Method::Get, "/api/v1/permissions/groups", None);
    assert_eq!(
        status,
        Status::Forbidden,
        "superuser must not reach /permissions"
    );
}

#[test]
fn relation_direction_rule_scopes_visibility() {
    let Some(h) = harness() else { return };
    let (_, body) = create_entity(&h, &uniq("95"), "ООО «Источник»", vec![]);
    let pid_a = body["data"]["public_id"].as_str().unwrap().to_string();
    let (_, body) = create_entity(&h, &uniq("96"), "ООО «Цель»", vec![]);
    let pid_b = body["data"]["public_id"].as_str().unwrap().to_string();
    let (status, _) = req(
        &h,
        Method::Post,
        &format!("/api/v1/entities/{pid_a}/relations"),
        Some(json!({"predicate": "owns", "target_entity_id": pid_b})),
    );
    assert_eq!(status, Status::Ok);

    // Deny reading `owns` in the incoming direction only.
    let (status, body) = create_group(
        &h,
        &uniq("deny_in_").to_lowercase(),
        "data",
        None,
        vec![json!({
            "effect":"deny","resource_type":"relation",
            "predicate":"owns","direction":"incoming","action":"read"
        })],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let login = uniq("dir_").to_lowercase();
    create_user(&h, &login, vec!["managers", "all_data", &gid]);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let owns_at = |body: &Value| {
        body["data"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter(|r| r["predicate"].as_str() == Some("owns"))
                    .count()
            })
            .unwrap_or(0)
    };
    // From the subject the edge is outgoing — still visible.
    let (status, body) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{pid_a}/relations"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(
        owns_at(&body) > 0,
        "outgoing edge must stay visible: {body}"
    );
    // From the target the same edge is incoming — hidden by the deny.
    let (status, body) = req(
        &u,
        Method::Get,
        &format!("/api/v1/entities/{pid_b}/relations"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(owns_at(&body), 0, "incoming edge must be denied: {body}");
    delete_group(&h, &gid);
}

#[test]
fn tokenize_mask_returns_stable_token() {
    let Some(h) = harness() else { return };
    let inn = uniq("48");
    let (_, body) = create_entity(&h, &inn, "ООО «Токен»", vec![]);
    let pid = body["data"]["public_id"].as_str().unwrap().to_string();

    let (status, body) = create_group(
        &h,
        &uniq("tok_inn_").to_lowercase(),
        "data",
        None,
        vec![json!({
            "effect":"allow","resource_type":"field",
            "type_alias":"legal_entity","field":"inn",
            "action":"read","mask":"tokenize"
        })],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let gid = body["data"]["id"].as_str().unwrap().to_string();
    let login = uniq("tok_").to_lowercase();
    create_user(&h, &login, vec!["managers", "all_data", &gid]);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(&u, Method::Get, &format!("/api/v1/entities/{pid}"), None);
    assert_eq!(status, Status::Ok, "{body}");
    let masked = body["data"]["fields"]["inn"].as_str().unwrap_or_default();
    assert!(
        masked.starts_with("tok_"),
        "tokenized mask expected: {masked}"
    );
    assert_ne!(masked, inn);
    delete_group(&h, &gid);
}

#[test]
fn capability_role_inherits_parent_grants() {
    let Some(h) = harness() else { return };
    // Parent capability role grants audit.read; child inherits (no own rules).
    let (status, body) = create_group(
        &h,
        &uniq("cap_parent_").to_lowercase(),
        "capability",
        None,
        vec![json!({"effect":"allow","resource_type":"capability","action":"audit.read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let parent_id = body["data"]["id"].as_str().unwrap().to_string();
    let (status, body) = create_group(
        &h,
        &uniq("cap_child_").to_lowercase(),
        "capability",
        Some(&parent_id),
        vec![],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let child_id = body["data"]["id"].as_str().unwrap().to_string();

    let login = uniq("capinh_").to_lowercase();
    let (status, _) = create_user(&h, &login, vec![&child_id]);
    assert_eq!(status, Status::Ok);

    let Some(u) = harness_as(&login, "test-pass-12345") else {
        return;
    };
    let (status, body) = req(&u, Method::Get, "/api/v1/audit?limit=1", None);
    assert_eq!(
        status,
        Status::Ok,
        "inherited audit.read must grant access: {body}"
    );
    // And /me/context must expose the inherited capability (menu parity).
    let (status, body) = req(&u, Method::Get, "/api/v1/me/context", None);
    assert_eq!(status, Status::Ok);
    let caps: Vec<&str> = body["data"]["capabilities"]
        .as_array()
        .map(|a| a.iter().filter_map(|c| c.as_str()).collect())
        .unwrap_or_default();
    assert!(
        caps.contains(&"audit.read"),
        "inherited cap missing: {caps:?}"
    );
    delete_group(&h, &child_id);
    delete_group(&h, &parent_id);
}

#[test]
fn self_disable_is_rejected() {
    let Some(h) = harness() else { return };
    let ctx_pid = {
        let (_, body) = req(&h, Method::Get, "/api/v1/me/context", None);
        body["data"]["principal_id"].as_str().unwrap().to_string()
    };
    let (status, _) = req(
        &h,
        Method::Delete,
        &format!("/api/v1/users/{ctx_pid}"),
        None,
    );
    assert_eq!(status, Status::UnprocessableEntity, "self-delete must fail");
    let (status, _) = req(
        &h,
        Method::Put,
        &format!("/api/v1/users/{ctx_pid}"),
        Some(json!({ "status": "disabled" })),
    );
    assert_eq!(
        status,
        Status::UnprocessableEntity,
        "self-disable via PUT must fail"
    );
}

// ---------------------------------------------------------------------------
// API tokens / bearer auth (docs/08 §2, docs/09 §3)
// ---------------------------------------------------------------------------

/// A client without a session cookie — bearer-only requests.
fn bare_client() -> Option<Client> {
    let state = state()?;
    Client::tracked(rocket_build(state)).ok()
}

fn bearer_req(
    client: &Client,
    method: Method,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> (Status, Value) {
    let mut r = client
        .req(method, path)
        .header(Header::new("Authorization", format!("Bearer {token}")));
    if let Some(b) = body {
        r = r.header(ContentType::JSON).body(b.to_string());
    }
    let resp = r.dispatch();
    let status = resp.status();
    let payload = resp.into_json().unwrap_or(Value::Null);
    (status, payload)
}

#[test]
fn service_account_bearer_full_lifecycle() {
    let Some(h) = harness() else { return };
    let login = uniq("svc_").to_lowercase();
    // Service account: no password, viewer-equivalent role (entity.read).
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/users",
        Some(json!({
            "login": login,
            "display_name": format!("Svc {login}"),
            "is_service_account": true,
            "groups": ["analysts"],
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["is_service_account"], true);
    let pid = body["data"]["principal_id"].as_str().unwrap().to_string();

    // Password-less service account cannot open a UI session.
    let login_resp = h
        .client
        .post("/api/v1/auth/login")
        .header(ContentType::JSON)
        .body(json!({"username": login, "password": "anything-long-enough"}).to_string())
        .dispatch();
    assert_eq!(login_resp.status(), Status::Unauthorized);

    // Issue a token.
    let (status, body) = req(
        &h,
        Method::Post,
        &format!("/api/v1/users/{pid}/tokens"),
        Some(json!({"name": "ci-bot"})),
    );
    assert_eq!(status, Status::Ok, "{body}");
    let token = body["data"]["token"].as_str().unwrap().to_string();
    let token_id = body["data"]["token_id"].as_str().unwrap().to_string();
    assert!(token.starts_with("egr_"), "{token}");
    assert_eq!(token.len(), 4 + 64);

    // Token listed without the secret material.
    let (status, body) = req(
        &h,
        Method::Get,
        &format!("/api/v1/users/{pid}/tokens"),
        None,
    );
    assert_eq!(status, Status::Ok);
    let rows = body["data"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["token_id"], token_id);
    assert!(
        rows[0].get("token").is_none(),
        "secret must not be listable"
    );
    assert!(rows[0].get("token_hash").is_none());

    // Bearer auth on a cookie-free client resolves the right principal and
    // the role-derived capability set.
    let Some(bare) = bare_client() else { return };
    let (status, body) = bearer_req(&bare, Method::Get, "/api/v1/me/context", &token, None);
    assert_eq!(status, Status::Ok, "{body}");
    assert_eq!(body["data"]["principal_id"], pid);
    assert_eq!(body["data"]["auth_strength"], "api_token");
    let caps: Vec<&str> = body["data"]["capabilities"]
        .as_array()
        .map(|a| a.iter().filter_map(|c| c.as_str()).collect())
        .unwrap_or_default();
    assert!(caps.contains(&"entity.read"), "caps: {caps:?}");

    // Bearer works for a real data endpoint; mutations need no CSRF.
    let (status, body) = bearer_req(&bare, Method::Get, "/api/v1/entities?limit=1", &token, None);
    assert_eq!(status, Status::Ok, "{body}");

    // viewer has no audit.read — capability gate still applies over bearer.
    let (status, _) = bearer_req(&bare, Method::Get, "/api/v1/audit?limit=1", &token, None);
    assert_eq!(status, Status::Forbidden);

    // Revocation kills the token.
    let (status, body) = req(
        &h,
        Method::Delete,
        &format!("/api/v1/users/{pid}/tokens/{token_id}"),
        None,
    );
    assert_eq!(status, Status::Ok, "{body}");
    let (status, _) = bearer_req(&bare, Method::Get, "/api/v1/me/context", &token, None);
    assert_eq!(status, Status::Unauthorized);

    // Cleanup: disable the service account.
    let _ = req(&h, Method::Delete, &format!("/api/v1/users/{pid}"), None);
}

#[test]
fn bearer_token_inherits_parent_role_grants() {
    let Some(h) = harness() else { return };
    // Parent capability role grants user.read; child inherits it.
    let (status, body) = create_group(
        &h,
        &uniq("bt_parent_").to_lowercase(),
        "capability",
        None,
        vec![json!({"effect":"allow","resource_type":"capability","action":"user.read"})],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let parent_id = body["data"]["id"].as_str().unwrap().to_string();
    let (status, body) = create_group(
        &h,
        &uniq("bt_child_").to_lowercase(),
        "capability",
        Some(&parent_id),
        vec![],
    );
    assert_eq!(status, Status::Ok, "{body}");
    let child_id = body["data"]["id"].as_str().unwrap().to_string();

    let login = uniq("svc_inh_").to_lowercase();
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/users",
        Some(json!({
            "login": login, "display_name": login,
            "is_service_account": true, "groups": [child_id],
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    let pid = body["data"]["principal_id"].as_str().unwrap().to_string();

    let (status, body) = req(
        &h,
        Method::Post,
        &format!("/api/v1/users/{pid}/tokens"),
        Some(json!({"name": "inh"})),
    );
    assert_eq!(status, Status::Ok);
    let token = body["data"]["token"].as_str().unwrap().to_string();

    let Some(bare) = bare_client() else { return };
    // Inherited user.read reaches the bearer token's principal.
    let (status, body) = bearer_req(&bare, Method::Get, "/api/v1/users?limit=1", &token, None);
    assert_eq!(status, Status::Ok, "inherited grant over bearer: {body}");

    delete_group(&h, &child_id);
    delete_group(&h, &parent_id);
    let _ = req(&h, Method::Delete, &format!("/api/v1/users/{pid}"), None);
}

#[test]
fn bearer_token_rejected_when_invalid_or_expired() {
    let Some(bare) = bare_client() else { return };
    // Garbage token.
    let (status, _) = bearer_req(
        &bare,
        Method::Get,
        "/api/v1/me/context",
        "egr_deadbeef",
        None,
    );
    assert_eq!(status, Status::Unauthorized);
    let bogus = format!("egr_{}", "0".repeat(64));
    let (status, _) = bearer_req(&bare, Method::Get, "/api/v1/me/context", &bogus, None);
    assert_eq!(status, Status::Unauthorized);
}

#[test]
fn disabling_account_revokes_its_tokens() {
    let Some(h) = harness() else { return };
    let login = uniq("svc_rev_").to_lowercase();
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/users",
        Some(json!({
            "login": login, "display_name": login,
            "is_service_account": true, "groups": ["analysts"],
        })),
    );
    assert_eq!(status, Status::Ok, "{body}");
    let pid = body["data"]["principal_id"].as_str().unwrap().to_string();
    let (status, body) = req(
        &h,
        Method::Post,
        &format!("/api/v1/users/{pid}/tokens"),
        Some(json!({"name": "x"})),
    );
    assert_eq!(status, Status::Ok);
    let token = body["data"]["token"].as_str().unwrap().to_string();

    let Some(bare) = bare_client() else { return };
    let (status, _) = bearer_req(&bare, Method::Get, "/api/v1/me/context", &token, None);
    assert_eq!(status, Status::Ok);

    let (status, _) = req(&h, Method::Delete, &format!("/api/v1/users/{pid}"), None);
    assert_eq!(status, Status::Ok);
    let (status, _) = bearer_req(&bare, Method::Get, "/api/v1/me/context", &token, None);
    assert_eq!(
        status,
        Status::Unauthorized,
        "disabled principal's token must die"
    );
}

/// Regression: `/schemas/published` must serialize the owning catalog
/// (hydrated from `core.catalog`, not trusted from JSON content), and a
/// draft's `base_revision` must belong to the resolved catalog — pinning a
/// revision from a foreign stream is rejected (docs/28 §3).
#[test]
fn schema_published_catalog_hydration_and_cross_catalog_base() {
    let Some(h) = harness() else { return };

    // 1. Every published record exposes its owning catalog alias.
    let (status, body) = req(&h, Method::Get, "/api/v1/schemas/published", None);
    assert_eq!(status, Status::Ok, "{body}");
    let all = body["data"].as_array().cloned().unwrap_or_default();
    assert!(!all.is_empty(), "no published revisions in demo data");
    for rec in &all {
        assert!(
            rec["catalog"].as_str().map(|c| !c.is_empty()).unwrap_or(false),
            "published revision missing catalog: {rec}"
        );
    }

    // 2. Revision of the general stream, plus a fresh catalog that owns no
    //    revisions — independent of whichever demo catalogs happen to exist.
    let mut by_catalog: std::collections::HashMap<String, String> = Default::default();
    for rec in &all {
        let cat = rec["catalog"].as_str().unwrap_or_default().to_string();
        let rev = rec["revision_id"].as_str().unwrap_or_default().to_string();
        if !cat.is_empty() && !rev.is_empty() {
            by_catalog.entry(cat).or_insert(rev);
        }
    }
    let general_rev = by_catalog
        .get("general")
        .cloned()
        .expect("general catalog must have a published revision");
    let alias = uniq("pgcat_").to_lowercase();
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/catalogs",
        Some(json!({"alias": alias, "name": alias, "description": "pg_scenarios"})),
    );
    assert!(status == Status::Ok || status == Status::Created, "{body}");

    // 3. A draft in the fresh catalog pinned to general's revision → 400.
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/schemas/drafts",
        Some(json!({
            "catalog": alias,
            "name": "pg cross-catalog",
            "base_revision": general_rev,
        })),
    );
    assert_eq!(
        status,
        Status::BadRequest,
        "cross-catalog base_revision accepted: {body}"
    );

    // 4. Same catalog without a base succeeds — then clean the draft and the
    //    catalog up.
    let (status, body) = req(
        &h,
        Method::Post,
        "/api/v1/schemas/drafts",
        Some(json!({
            "catalog": alias,
            "name": "pg same-catalog",
        })),
    );
    assert_eq!(status, Status::Created, "{body}");
    let draft_id = body["data"]["revision_id"].as_str().unwrap().to_string();
    let etag = body["data"]["etag"].as_str().unwrap().to_string();
    let delete_path = format!("/api/v1/schemas/drafts/{draft_id}");
    let r = h
        .client
        .req(Method::Delete, &delete_path)
        .header(Header::new("X-CSRF-Token", h.csrf.clone()))
        .header(Header::new("If-Match", etag))
        .dispatch();
    assert_eq!(r.status(), Status::Ok, "draft cleanup failed");
    let (status, body) = req(&h, Method::Post, &format!("/api/v1/catalogs/{alias}/retire"), None);
    assert_eq!(status, Status::Ok, "catalog cleanup failed: {body}");
}
