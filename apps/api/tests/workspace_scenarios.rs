use enginer_api::{rocket_build, ApiConfig, AppState};
use rocket::http::{ContentType, Cookie, Method, Status};
use rocket::local::blocking::Client;
use serde_json::{json, Value};

// NOTE: these scenarios exercise the API contract end-to-end. They now
// require a live PostgreSQL-backed AppState (auth, RLS, operational tables),
// so they are ignored in the default test run until a DB-backed test
// harness is wired up.

fn client() -> Client {
    Client::tracked(rocket_build(AppState::new(
        ApiConfig::default(),
        None,
        None,
    )))
    .expect("workspace API test client")
}

fn request(client: &Client, method: Method, path: &str, body: Option<Value>) -> (Status, Value) {
    let mut request = client
        .req(method, path)
        .cookie(Cookie::new("session", "workspace-test"));
    if let Some(body) = body {
        request = request.header(ContentType::JSON).body(body.to_string());
    }
    let response = request.dispatch();
    let status = response.status();
    let payload = response.into_json().unwrap_or(Value::Null);
    (status, payload)
}

fn create_legal_entity_body(name: &str, inn: &str, ogrn: &str, identity_mode: &str) -> Value {
    json!({
        "type_id": "legal_entity",
        "schema_revision": 1,
        "identity_mode": identity_mode,
        "statements": [
            { "predicate_id": "full_name", "value": name },
            { "predicate_id": "short_name", "value": name },
            { "predicate_id": "has_tax_id", "value": inn },
            { "predicate_id": "has_ogrn", "value": ogrn }
        ]
    })
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn object_create_search_edit_and_revision_conflict_scenario() {
    let client = client();

    let (status, unrelated) = request(
        &client,
        Method::Post,
        "/api/v1/entities/query",
        Some(json!({
            "from": { "type_id": "legal_entity" },
            "where": { "comparison": {
                "field": { "predicate_id": "has_tax_id" },
                "op": "eq",
                "value": "9999999999"
            }},
            "page": { "size": 20 }
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(unrelated["data"]["rows"].as_array().unwrap().len(), 0);

    let (status, created) = request(
        &client,
        Method::Post,
        "/api/v1/entities",
        Some(create_legal_entity_body(
            "ООО Новый контрагент",
            "7812345678",
            "1067812345678",
            "return_existing",
        )),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(created["data"]["outcome"], "created");
    let public_id = created["data"]["public_id"].as_str().unwrap();

    let (status, found) = request(
        &client,
        Method::Post,
        "/api/v1/entities/query",
        Some(json!({
            "from": { "type_id": "legal_entity" },
            "where": { "comparison": {
                "field": { "predicate_id": "has_tax_id" },
                "op": "eq",
                "value": "7812345678"
            }},
            "page": { "size": 20 }
        })),
    );
    assert_eq!(status, Status::Ok);
    let rows = found["data"]["rows"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["public_id"], public_id);

    let (status, updated) = request(
        &client,
        Method::Put,
        &format!("/api/v1/entities/{public_id}"),
        Some(json!({
            "revision": 1,
            "fields": [
                { "predicate_id": "full_name", "value": "ООО Новый контрагент — обновлён" },
                { "predicate_id": "short_name", "value": "ООО НК" }
            ]
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(updated["data"]["revision"], 2);
    assert_eq!(updated["data"]["display_name"], "ООО НК");
    assert_eq!(updated["data"]["fields"]["has_tax_id"], "7812345678");

    let (status, history) = request(
        &client,
        Method::Get,
        &format!("/api/v1/entities/{public_id}/history"),
        None,
    );
    assert_eq!(status, Status::Ok);
    let revisions = history["data"].as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[1]["operation"], "update");
    assert!(revisions[1]["changes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|change| change["alias"] == "short_name" && change["new_value"] == "ООО НК"));

    let (status, _) = request(
        &client,
        Method::Put,
        &format!("/api/v1/entities/{public_id}"),
        Some(json!({
            "revision": 1,
            "fields": [{ "predicate_id": "short_name", "value": "Устаревшее изменение" }]
        })),
    );
    assert_eq!(status, Status::Conflict);

    let (status, _) = request(
        &client,
        Method::Put,
        &format!("/api/v1/entities/{public_id}"),
        Some(json!({
            "revision": 2,
            "fields": [{ "predicate_id": "has_tax_id", "value": "7701234567" }]
        })),
    );
    assert_eq!(status, Status::Conflict);

    let (status, unchanged) = request(
        &client,
        Method::Put,
        &format!("/api/v1/entities/{public_id}"),
        Some(json!({
            "revision": 2,
            "fields": [
                { "predicate_id": "full_name", "value": "ООО Новый контрагент — обновлён" },
                { "predicate_id": "short_name", "value": "ООО НК" }
            ]
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(unchanged["data"]["revision"], 2);

    let (_, types) = request(&client, Method::Get, "/api/v1/schemas/types", None);
    let legal_type = types["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|schema_type| schema_type["id"] == "legal_entity")
        .unwrap();
    assert_eq!(legal_type["entity_count"], 3);
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn duplicate_identity_modes_only_match_real_identity_values() {
    let client = client();
    let existing = create_legal_entity_body(
        "Название не должно влиять на сильный идентификатор",
        "7701234567",
        "1999999999999",
        "return_existing",
    );
    let (status, deduplicated) = request(
        &client,
        Method::Post,
        "/api/v1/entities",
        Some(existing.clone()),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(deduplicated["data"]["outcome"], "deduplicated");
    assert_eq!(deduplicated["data"]["public_id"], "ent-001");

    let mut rejected = existing.clone();
    rejected["identity_mode"] = json!("reject");
    let (status, _) = request(&client, Method::Post, "/api/v1/entities", Some(rejected));
    assert_eq!(status, Status::Conflict);

    let mut candidate = existing;
    candidate["identity_mode"] = json!("create_candidate");
    let (status, created) = request(&client, Method::Post, "/api/v1/entities", Some(candidate));
    assert_eq!(status, Status::Ok);
    assert_eq!(created["data"]["outcome"], "created");
    assert_ne!(created["data"]["public_id"], "ent-001");

    let (status, same_name) = request(
        &client,
        Method::Post,
        "/api/v1/entities",
        Some(create_legal_entity_body(
            "ООО Ромашка",
            "9999999999",
            "1999999999998",
            "return_existing",
        )),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(same_name["data"]["outcome"], "created");
    assert_ne!(same_name["data"]["public_id"], "ent-001");
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn type_predicate_relation_and_graph_scenario() {
    let client = client();
    let type_body = json!({
        "type_id": "vendor",
        "display_name": "Поставщик",
        "description": "Внешний поставщик",
        "entity_class": "data",
        "fields": [
            { "alias": "name", "label": "Название", "kind": "text", "required": true, "identity": true },
            { "alias": "vendor_code", "label": "Код", "kind": "text", "required": false, "identity": true }
        ]
    });
    let (status, _) = request(
        &client,
        Method::Post,
        "/api/v1/schemas/types",
        Some(type_body.clone()),
    );
    assert_eq!(status, Status::Ok);

    let mut updated_type = type_body;
    updated_type["display_name"] = json!("Проверенный поставщик");
    updated_type["description"] = json!("Поставщик после проверки");
    let (status, updated) = request(
        &client,
        Method::Put,
        "/api/v1/schemas/types/vendor",
        Some(updated_type),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(updated["data"]["version"], 2);
    assert_eq!(updated["data"]["display_name"], "Проверенный поставщик");

    let (status, created) = request(
        &client,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": "vendor",
            "schema_revision": 2,
            "identity_mode": "return_existing",
            "statements": [
                { "predicate_id": "name", "value": "ООО Снабжение" },
                { "predicate_id": "vendor_code", "value": "SUP-42" }
            ]
        })),
    );
    assert_eq!(status, Status::Ok);
    let vendor_id = created["data"]["public_id"].as_str().unwrap();

    let predicate = json!({
        "alias": "supplies_to",
        "label": "Поставляет для",
        "is_edge": true,
        "direction": "directed",
        "domain_type": "vendor",
        "range_type": "legal_entity",
        "scalar_kind": "text",
        "min_cardinality": 0,
        "max_cardinality": -1,
        "participates_in_identity": false,
        "confidentiality_class": "public",
        "inverse_label": "Получает от"
    });
    let (status, _) = request(
        &client,
        Method::Post,
        "/api/v1/schemas/predicates",
        Some(predicate.clone()),
    );
    assert_eq!(status, Status::Ok);

    let mut updated_predicate = predicate;
    updated_predicate["label"] = json!("Регулярно поставляет для");
    let (status, updated) = request(
        &client,
        Method::Put,
        "/api/v1/schemas/predicates/supplies_to",
        Some(updated_predicate),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(updated["data"]["label"], "Регулярно поставляет для");

    let (status, relation) = request(
        &client,
        Method::Post,
        &format!("/api/v1/entities/{vendor_id}/relations"),
        Some(json!({
            "predicate": "supplies_to",
            "target_entity_id": "ent-003",
            "valid_from": "2026-10-01T00:00:00Z",
            "source": "manual"
        })),
    );
    assert_eq!(status, Status::Ok);
    let relation_id = relation["data"]["relation_id"].as_str().unwrap();

    let (status, historical_slice) = request(
        &client,
        Method::Get,
        &format!("/api/v1/entities/{vendor_id}/relations?as_of=2020-01-01"),
        None,
    );
    assert_eq!(status, Status::Ok);
    assert!(historical_slice["data"].as_array().unwrap().is_empty());

    let (status, historical_graph) = request(
        &client,
        Method::Post,
        &format!("/api/v1/entities/{vendor_id}/graph"),
        Some(json!({ "max_depth": 2, "direction": "both", "as_of": "2020-01-01" })),
    );
    assert_eq!(status, Status::Ok);
    assert!(historical_graph["data"]["edges"]
        .as_array()
        .unwrap()
        .is_empty());

    let (status, graph) = request(
        &client,
        Method::Post,
        &format!("/api/v1/entities/{vendor_id}/graph"),
        Some(json!({ "max_depth": 2, "direction": "both" })),
    );
    assert_eq!(status, Status::Ok);
    assert!(graph["data"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|node| node["id"] == "ent-003"));
    assert!(graph["data"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .any(|edge| edge["predicate"] == "supplies_to"));

    let (status, _) = request(
        &client,
        Method::Delete,
        "/api/v1/schemas/predicates/supplies_to",
        None,
    );
    assert_eq!(status, Status::Conflict);

    let (status, _) = request(
        &client,
        Method::Delete,
        &format!("/api/v1/entities/{vendor_id}/relations/{relation_id}"),
        None,
    );
    assert_eq!(status, Status::Ok);
    let (status, _) = request(
        &client,
        Method::Delete,
        "/api/v1/schemas/predicates/supplies_to",
        None,
    );
    assert_eq!(status, Status::Ok);

    let (status, _) = request(
        &client,
        Method::Delete,
        "/api/v1/schemas/types/vendor",
        None,
    );
    assert_eq!(status, Status::Conflict);
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn request_creation_resolves_and_persists_selected_target() {
    let client = client();
    let (status, created) = request(
        &client,
        Method::Post,
        "/api/v1/requests",
        Some(json!({
            "target_entity_id": "ent-003",
            "requested_product": "Проверка лимита",
            "requested_limit": 2_500_000,
            "urgency": "high"
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(created["data"]["target_display_name"], "АО Вектор");

    let (status, listed) = request(&client, Method::Get, "/api/v1/requests", None);
    assert_eq!(status, Status::Ok);
    let request_id = created["data"]["request_id"].as_str().unwrap();
    assert!(listed["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|item| item["request_id"] == request_id));
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn document_task_assessment_and_permission_lifecycle() {
    let client = client();

    let (status, uploaded) = request(
        &client,
        Method::Post,
        "/api/v1/entities/ent-001/documents",
        Some(json!({
            "title": "Тестовый документ",
            "kind": "contract",
            "file_name": "contract.txt",
            "content_type": "text/plain",
            "bytes": [69, 110, 103, 105, 110, 101, 82]
        })),
    );
    assert_eq!(status, Status::Ok);
    let document_id = uploaded["data"]["id"].as_str().unwrap();
    let response = client
        .get(format!(
            "/api/v1/entities/ent-001/documents/{document_id}/download"
        ))
        .cookie(Cookie::new("session", "workspace-test"))
        .dispatch();
    assert_eq!(response.status(), Status::Ok);
    assert_eq!(response.into_bytes().unwrap(), b"EngineR");
    let (status, _) = request(
        &client,
        Method::Delete,
        &format!("/api/v1/entities/ent-001/documents/{document_id}"),
        None,
    );
    assert_eq!(status, Status::Ok);

    let (status, task) = request(
        &client,
        Method::Post,
        "/api/v1/entities/ent-001/tasks",
        Some(json!({
            "title": "Проверить документы",
            "kind": "check_execution",
            "description": "Пользовательский сценарий"
        })),
    );
    assert_eq!(status, Status::Ok);
    let task_id = task["data"]["task_id"].as_str().unwrap();
    let (status, started) = request(
        &client,
        Method::Post,
        &format!("/api/v1/tasks/{task_id}/start"),
        Some(json!({})),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(started["data"]["status"], "in_progress");
    let (status, completed) = request(
        &client,
        Method::Post,
        &format!("/api/v1/tasks/{task_id}/complete"),
        Some(json!({ "result": {} })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(completed["data"]["status"], "completed");

    let (status, assessment) = request(
        &client,
        Method::Post,
        "/api/v1/entities/ent-001/assessment",
        Some(json!({
            "force": true,
            "desired_level": "high",
            "decision": "escalate",
            "rationale": "Нужна ручная проверка"
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(assessment["data"]["risk_level"], "high");
    assert_eq!(assessment["data"]["approval_pending"], true);

    let group_body = json!({
        "name": "Аналитики",
        "description": "Доступ для анализа",
        "principals": ["analyst@example.test"],
        "rules": [{
            "effect": "allow",
            "resource_type": "type",
            "action": "read"
        }]
    });
    let (status, group) = request(
        &client,
        Method::Post,
        "/api/v1/permissions/groups",
        Some(group_body.clone()),
    );
    assert_eq!(status, Status::Ok);
    let group_id = group["data"]["id"].as_str().unwrap();
    let mut updated_group = group_body;
    updated_group["description"] = json!("Обновлённый доступ");
    let (status, updated) = request(
        &client,
        Method::Put,
        &format!("/api/v1/permissions/groups/{group_id}"),
        Some(updated_group),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(updated["data"]["description"], "Обновлённый доступ");
    let (status, _) = request(
        &client,
        Method::Delete,
        &format!("/api/v1/permissions/groups/{group_id}"),
        None,
    );
    assert_eq!(status, Status::Ok);
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn staged_file_report_workflow_and_merge_lifecycle() {
    let client = client();

    let (status, initialized) = request(
        &client,
        Method::Post,
        "/api/v1/files/upload/init",
        Some(json!({
            "filename": "payload.bin",
            "content_type": "application/octet-stream",
            "expected_size": 7,
            "expected_hash": "5972af04a7ce774c08e547514f6069649d4510d890aa6b2c3630865a2a686d32",
            "hash_algorithm": "sha256",
            "chunk_size": 4
        })),
    );
    assert_eq!(status, Status::Ok);
    let upload_id = initialized["data"]["upload_id"].as_str().unwrap();
    for (index, bytes) in [(0, b"Eng".as_slice()), (1, b"ineR".as_slice())] {
        let response = client
            .put(format!("/api/v1/files/upload/{upload_id}/chunks/{index}"))
            .cookie(Cookie::new("session", "workspace-test"))
            .header(ContentType::Binary)
            .body(bytes)
            .dispatch();
        if index == 0 {
            assert_eq!(response.status(), Status::UnprocessableEntity);
        }
    }
    for (index, bytes) in [(0, b"Engi".as_slice()), (1, b"neR".as_slice())] {
        let response = client
            .put(format!("/api/v1/files/upload/{upload_id}/chunks/{index}"))
            .cookie(Cookie::new("session", "workspace-test"))
            .header(ContentType::Binary)
            .body(bytes)
            .dispatch();
        assert_eq!(response.status(), Status::Ok);
    }
    let (status, completed) = request(
        &client,
        Method::Post,
        &format!("/api/v1/files/upload/{upload_id}/complete"),
        Some(json!({
            "final_hash": "5972af04a7ce774c08e547514f6069649d4510d890aa6b2c3630865a2a686d32",
            "total_size": 7
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(completed["data"]["status"], "completed");
    let object_id = completed["data"]["object_id"].as_str().unwrap();
    let response = client
        .get(format!("/api/v1/files/{object_id}/download"))
        .cookie(Cookie::new("session", "workspace-test"))
        .dispatch();
    assert_eq!(response.into_bytes().unwrap(), b"EngineR");

    let (status, report) = request(
        &client,
        Method::Post,
        "/api/v1/reports",
        Some(json!({
            "entity_id": "ent-001",
            "template": "due_diligence",
            "format": "json"
        })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(report["data"]["status"], "ready");
    assert!(report["data"]["download_url"].as_str().is_some());

    let (status, workflow) = request(
        &client,
        Method::Post,
        "/api/v1/workflows",
        Some(json!({
            "definition_id": "request_processing",
            "target_entity_id": "ent-001",
            "variables": { "source": "test" }
        })),
    );
    assert_eq!(status, Status::Ok);
    let workflow_id = workflow["data"]["instance_id"].as_str().unwrap();
    let (status, transitioned) = request(
        &client,
        Method::Post,
        &format!("/api/v1/workflows/{workflow_id}/transition"),
        Some(json!({ "to": "completed", "trigger": "finish" })),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(transitioned["data"]["terminated"], true);

    let (status, loser) = request(
        &client,
        Method::Post,
        "/api/v1/entities",
        Some(json!({
            "type_id": "person",
            "schema_revision": 1,
            "identity_mode": "return_existing",
            "statements": [
                { "predicate_id": "full_name", "value": "Иван Иванов — дубль" },
                { "predicate_id": "inn", "value": "500100732259" }
            ]
        })),
    );
    assert_eq!(status, Status::Ok);
    let loser_id = loser["data"]["public_id"].as_str().unwrap();
    let merge_body = json!({ "loser_public_id": loser_id, "reason": "Подтверждённый дубль" });
    let (status, merged) = request(
        &client,
        Method::Post,
        "/api/v1/entities/ent-002/merge",
        Some(merge_body.clone()),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(merged["data"]["idempotent"], false);
    let (status, merged_again) = request(
        &client,
        Method::Post,
        "/api/v1/entities/ent-002/merge",
        Some(merge_body),
    );
    assert_eq!(status, Status::Ok);
    assert_eq!(merged_again["data"]["idempotent"], true);
}

#[test]
#[ignore = "requires a live PostgreSQL AppState"]
fn audit_filters_checkpoint_verification_and_retention_persist() {
    let client = client();

    let unauthorized = client.get("/api/v1/audit").dispatch();
    assert_eq!(unauthorized.status(), Status::Unauthorized);

    let (status, filtered) = request(
        &client,
        Method::Get,
        "/api/v1/audit?kind=decision&actor=admin&limit=1",
        None,
    );
    assert_eq!(status, Status::Ok);
    let entries = filtered["data"].as_array().unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["action"], "entity.read");

    let (status, cursor_page) =
        request(&client, Method::Get, "/api/v1/audit?cursor=3&limit=1", None);
    assert_eq!(status, Status::Ok);
    assert_eq!(cursor_page["data"][0]["sequence"], 2);

    let (status, verified) = request(&client, Method::Post, "/api/v1/audit/verify", None);
    assert_eq!(status, Status::Ok);
    assert_eq!(verified["data"]["valid"], true);
    assert_eq!(verified["data"]["entry_count"], 3);

    let (status, checkpoint) = request(&client, Method::Post, "/api/v1/audit/checkpoint", None);
    assert_eq!(status, Status::Ok);
    let checkpoint_id = checkpoint["data"]["checkpoint_id"].as_i64().unwrap();
    assert!(checkpoint_id > 1);
    let (status, latest) = request(&client, Method::Get, "/api/v1/audit/checkpoint", None);
    assert_eq!(status, Status::Ok);
    assert_eq!(latest["data"]["checkpoint_id"], checkpoint_id);

    let policy = json!({
        "retention_days": 730,
        "legal_hold": true,
        "archive_before_delete": false,
        "archive_url": null
    });
    let (status, _) = request(
        &client,
        Method::Put,
        "/api/v1/audit/retention",
        Some(policy.clone()),
    );
    assert_eq!(status, Status::Ok);
    let (status, retained) = request(&client, Method::Get, "/api/v1/audit/retention", None);
    assert_eq!(status, Status::Ok);
    assert_eq!(retained["data"], policy);

    let (status, _) = request(
        &client,
        Method::Put,
        "/api/v1/audit/retention",
        Some(json!({
            "retention_days": 0,
            "legal_hold": false,
            "archive_before_delete": false,
            "archive_url": null
        })),
    );
    assert_eq!(status, Status::UnprocessableEntity);
}
