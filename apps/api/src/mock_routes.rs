//! Mock data routes for Docker demo.
//!
//! Provides mock responses for audit, checks, integrations, entities,
//! schemas, connectors, files, and other endpoints that require a DB.
//!
//! In production these are backed by real repositories.

use rocket::serde::json::Json;
use rocket::{get, post, put, routes};
use serde::Serialize;

/// API envelope wrapper.
#[derive(Debug, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: serde_json::Value,
}

fn meta() -> serde_json::Value {
    serde_json::json!({ "request_id": "mock" })
}

// --- Audit ---

#[derive(Debug, Serialize)]
pub struct AuditRecordMock {
    pub audit_id: i64,
    pub sequence: i64,
    pub occurred_at: String,
    pub kind: String,
    pub actor: String,
    pub effective_actor: Option<String>,
    pub action: String,
    pub resource_kind: String,
    pub resource_id: Option<String>,
    pub effect: Option<String>,
    pub reason_ids: Vec<String>,
    pub entry_hash: String,
    pub previous_hash: String,
    pub request_id: Option<String>,
    pub correlation_id: Option<String>,
    pub policy_revision: Option<i64>,
    pub query_fingerprint: Option<String>,
    pub row_count: Option<i64>,
    pub bytes: Option<i64>,
    pub client: Option<String>,
    pub service: Option<String>,
    pub metadata: serde_json::Value,
}

#[get("/audit?<kind>&<actor>&<action>&<resource_kind>&<resource_id>&<effect>&<from>&<to>&<limit>&<cursor>")]
pub async fn query_audit(
    kind: Option<String>,
    actor: Option<String>,
    action: Option<String>,
    resource_kind: Option<String>,
    resource_id: Option<String>,
    effect: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: Option<u32>,
    cursor: Option<i64>,
) -> Json<ApiEnvelope<Vec<AuditRecordMock>>> {
    let _ = (kind, actor, action, resource_kind, resource_id, effect, from, to, limit, cursor);
    let entries = vec![
        AuditRecordMock {
            audit_id: 1,
            sequence: 1,
            occurred_at: "2026-08-03T12:00:00Z".to_string(),
            kind: "authentication".to_string(),
            actor: "admin".to_string(),
            effective_actor: None,
            action: "login".to_string(),
            resource_kind: "session".to_string(),
            resource_id: Some("sess-001".to_string()),
            effect: Some("allow".to_string()),
            reason_ids: vec![],
            entry_hash: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
            previous_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            request_id: Some("req-001".to_string()),
            correlation_id: None,
            policy_revision: Some(1),
            query_fingerprint: None,
            row_count: None,
            bytes: None,
            client: Some("127.0.0.1".to_string()),
            service: Some("enginer-api".to_string()),
            metadata: serde_json::json!({}),
        },
        AuditRecordMock {
            audit_id: 2,
            sequence: 2,
            occurred_at: "2026-08-03T12:05:00Z".to_string(),
            kind: "decision".to_string(),
            actor: "admin".to_string(),
            effective_actor: None,
            action: "entity.read".to_string(),
            resource_kind: "entity".to_string(),
            resource_id: Some("ent-001".to_string()),
            effect: Some("allow".to_string()),
            reason_ids: vec!["policy-001".to_string()],
            entry_hash: "b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3".to_string(),
            previous_hash: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
            request_id: Some("req-002".to_string()),
            correlation_id: None,
            policy_revision: Some(1),
            query_fingerprint: Some("fp-001".to_string()),
            row_count: Some(5),
            bytes: None,
            client: Some("127.0.0.1".to_string()),
            service: Some("enginer-api".to_string()),
            metadata: serde_json::json!({ "entity_type": "person" }),
        },
        AuditRecordMock {
            audit_id: 3,
            sequence: 3,
            occurred_at: "2026-08-03T12:10:00Z".to_string(),
            kind: "denied_request".to_string(),
            actor: "user-002".to_string(),
            effective_actor: None,
            action: "entity.create".to_string(),
            resource_kind: "entity".to_string(),
            resource_id: None,
            effect: Some("deny".to_string()),
            reason_ids: vec!["policy-002".to_string()],
            entry_hash: "c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4".to_string(),
            previous_hash: "b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3".to_string(),
            request_id: Some("req-003".to_string()),
            correlation_id: None,
            policy_revision: Some(1),
            query_fingerprint: None,
            row_count: None,
            bytes: None,
            client: Some("127.0.0.1".to_string()),
            service: Some("enginer-api".to_string()),
            metadata: serde_json::json!({ "reason": "missing_capability" }),
        },
    ];
    Json(ApiEnvelope { data: entries, meta: meta() })
}

#[derive(Debug, Serialize)]
pub struct CheckpointMock {
    pub checkpoint_id: i64,
    pub sequence: i64,
    pub entry_hash: String,
    pub created_at: String,
}

#[get("/audit/checkpoint")]
pub async fn latest_checkpoint() -> Json<ApiEnvelope<CheckpointMock>> {
    Json(ApiEnvelope {
        data: CheckpointMock {
            checkpoint_id: 1,
            sequence: 3,
            entry_hash: "c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4".to_string(),
            created_at: "2026-08-03T12:15:00Z".to_string(),
        },
        meta: meta(),
    })
}

#[post("/audit/checkpoint")]
pub async fn create_checkpoint() -> Json<ApiEnvelope<CheckpointMock>> {
    Json(ApiEnvelope {
        data: CheckpointMock {
            checkpoint_id: 2,
            sequence: 3,
            entry_hash: "c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4".to_string(),
            created_at: "2026-08-03T12:20:00Z".to_string(),
        },
        meta: meta(),
    })
}

#[derive(Debug, Serialize)]
pub struct ChainVerificationMock {
    pub valid: bool,
    pub entry_count: i64,
    pub last_sequence: i64,
}

#[get("/audit/verify")]
pub async fn verify_chain() -> Json<ApiEnvelope<ChainVerificationMock>> {
    Json(ApiEnvelope {
        data: ChainVerificationMock {
            valid: true,
            entry_count: 3,
            last_sequence: 3,
        },
        meta: meta(),
    })
}

#[derive(Debug, Serialize, serde::Deserialize)]
pub struct RetentionPolicyMock {
    pub retention_days: i32,
    pub legal_hold: bool,
    pub archive_before_delete: bool,
    pub archive_url: Option<String>,
}

#[get("/audit/retention")]
pub async fn get_retention() -> Json<ApiEnvelope<RetentionPolicyMock>> {
    Json(ApiEnvelope {
        data: RetentionPolicyMock {
            retention_days: 365,
            legal_hold: false,
            archive_before_delete: true,
            archive_url: Some("s3://enginer-archive".to_string()),
        },
        meta: meta(),
    })
}

#[put("/audit/retention", data = "<policy>")]
pub async fn put_retention(policy: Json<RetentionPolicyMock>) -> Json<ApiEnvelope<RetentionPolicyMock>> {
    Json(ApiEnvelope { data: policy.into_inner(), meta: meta() })
}

// --- Checks ---

#[derive(Debug, Serialize)]
pub struct CheckInstanceMock {
    pub check_id: String,
    pub request_id: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub evidence_count: i32,
    pub risk_level: Option<String>,
}

#[get("/checks?<status>&<kind>&<limit>&<cursor>")]
pub async fn query_checks(
    status: Option<String>,
    kind: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<CheckInstanceMock>>> {
    let _ = (status, kind, limit, cursor);
    let checks = vec![
        CheckInstanceMock {
            check_id: "chk-001".to_string(),
            request_id: "req-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            kind: "sanctions".to_string(),
            status: "completed".to_string(),
            started_at: "2026-08-03T10:00:00Z".to_string(),
            completed_at: Some("2026-08-03T10:05:00Z".to_string()),
            evidence_count: 3,
            risk_level: Some("low".to_string()),
        },
        CheckInstanceMock {
            check_id: "chk-002".to_string(),
            request_id: "req-002".to_string(),
            target_entity_id: "ent-002".to_string(),
            target_display_name: "Иван Иванов".to_string(),
            kind: "kyc".to_string(),
            status: "running".to_string(),
            started_at: "2026-08-03T11:00:00Z".to_string(),
            completed_at: None,
            evidence_count: 1,
            risk_level: None,
        },
        CheckInstanceMock {
            check_id: "chk-003".to_string(),
            request_id: "req-003".to_string(),
            target_entity_id: "ent-003".to_string(),
            target_display_name: "АО Вектор".to_string(),
            kind: "financial_health".to_string(),
            status: "pending".to_string(),
            started_at: "2026-08-03T12:00:00Z".to_string(),
            completed_at: None,
            evidence_count: 0,
            risk_level: None,
        },
    ];
    Json(ApiEnvelope { data: checks, meta: meta() })
}

// --- Connectors / Integrations ---

#[derive(Debug, Serialize)]
pub struct ConnectorInstanceMock {
    pub instance_id: String,
    pub connector_type: String,
    pub display_name: String,
    pub enabled: bool,
    pub trust_rank: String,
    pub last_health_check: Option<String>,
    pub status: String,
}

#[get("/connectors")]
pub async fn list_connectors() -> Json<ApiEnvelope<Vec<ConnectorInstanceMock>>> {
    let connectors = vec![
        ConnectorInstanceMock {
            instance_id: "conn-001".to_string(),
            connector_type: "api".to_string(),
            display_name: "ЕГРЮЛ API".to_string(),
            enabled: true,
            trust_rank: "Authoritative".to_string(),
            last_health_check: Some("2026-08-03T12:00:00Z".to_string()),
            status: "healthy".to_string(),
        },
        ConnectorInstanceMock {
            instance_id: "conn-002".to_string(),
            connector_type: "api".to_string(),
            display_name: "Санкционные списки".to_string(),
            enabled: true,
            trust_rank: "High".to_string(),
            last_health_check: Some("2026-08-03T11:30:00Z".to_string()),
            status: "healthy".to_string(),
        },
        ConnectorInstanceMock {
            instance_id: "conn-003".to_string(),
            connector_type: "file_import".to_string(),
            display_name: "Excel импорт".to_string(),
            enabled: false,
            trust_rank: "Medium".to_string(),
            last_health_check: None,
            status: "disabled".to_string(),
        },
    ];
    Json(ApiEnvelope { data: connectors, meta: meta() })
}

// --- Schemas ---

#[derive(Debug, Serialize)]
pub struct SchemaTypeMock {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub version: i32,
}

#[get("/schemas/types")]
pub async fn list_schema_types() -> Json<ApiEnvelope<Vec<SchemaTypeMock>>> {
    let types = vec![
        SchemaTypeMock {
            id: "person".to_string(),
            name: "person".to_string(),
            display_name: "Физическое лицо".to_string(),
            status: "published".to_string(),
            version: 1,
        },
        SchemaTypeMock {
            id: "legal_entity".to_string(),
            name: "legal_entity".to_string(),
            display_name: "Юридическое лицо".to_string(),
            status: "published".to_string(),
            version: 1,
        },
        SchemaTypeMock {
            id: "request".to_string(),
            name: "request".to_string(),
            display_name: "Заявка".to_string(),
            status: "published".to_string(),
            version: 1,
        },
    ];
    Json(ApiEnvelope { data: types, meta: meta() })
}

// --- Entities ---

#[derive(Debug, Serialize)]
pub struct EntityMock {
    pub public_id: String,
    pub entity_type: String,
    pub display_name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[get("/entities?<type>&<limit>&<cursor>")]
pub async fn query_entities(
    r#type: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<EntityMock>>> {
    let _ = (r#type, limit, cursor);
    let entities = vec![
        EntityMock {
            public_id: "ent-001".to_string(),
            entity_type: "legal_entity".to_string(),
            display_name: "ООО Ромашка".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-01T10:00:00Z".to_string(),
            updated_at: "2026-08-03T12:00:00Z".to_string(),
        },
        EntityMock {
            public_id: "ent-002".to_string(),
            entity_type: "person".to_string(),
            display_name: "Иван Иванов".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T10:00:00Z".to_string(),
            updated_at: "2026-08-03T11:00:00Z".to_string(),
        },
        EntityMock {
            public_id: "ent-003".to_string(),
            entity_type: "legal_entity".to_string(),
            display_name: "АО Вектор".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T14:00:00Z".to_string(),
            updated_at: "2026-08-03T10:00:00Z".to_string(),
        },
    ];
    Json(ApiEnvelope { data: entities, meta: meta() })
}

// --- Entity query (POST /entities/query) ---

#[derive(Debug, Serialize)]
pub struct QueryResponseMock {
    pub rows: Vec<serde_json::Value>,
    pub cursor: Option<String>,
    pub total_estimate: Option<i64>,
}

#[post("/entities/query", data = "<body>")]
pub async fn query_entities_post(body: String) -> Json<ApiEnvelope<QueryResponseMock>> {
    // Parse the query body to extract type_id for filtering
    let type_id = serde_json::from_str::<serde_json::Value>(&body)
        .ok()
        .and_then(|v| {
            v.get("from")
                .and_then(|f| f.get("type_id"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    let rows = match type_id.as_str() {
        "person" => vec![
            serde_json::json!({ "public_id": "ent-002", "display_name": "Иван Иванов", "status": "active" }),
            serde_json::json!({ "public_id": "ent-004", "display_name": "Пётр Петров", "status": "active" }),
        ],
        "legal_entity" => vec![
            serde_json::json!({ "public_id": "ent-001", "display_name": "ООО Ромашка", "status": "active" }),
            serde_json::json!({ "public_id": "ent-003", "display_name": "АО Вектор", "status": "active" }),
        ],
        _ => vec![
            serde_json::json!({ "public_id": "ent-001", "display_name": "ООО Ромашка", "status": "active" }),
            serde_json::json!({ "public_id": "ent-002", "display_name": "Иван Иванов", "status": "active" }),
            serde_json::json!({ "public_id": "ent-003", "display_name": "АО Вектор", "status": "active" }),
        ],
    };
    Json(ApiEnvelope {
        data: QueryResponseMock {
            rows,
            cursor: None,
            total_estimate: Some(3),
        },
        meta: meta(),
    })
}

// --- Get single entity ---

#[get("/entities/<public_id>")]
pub async fn get_entity(public_id: String) -> Json<ApiEnvelope<EntityMock>> {
    let entity = match public_id.as_str() {
        "ent-001" => EntityMock {
            public_id: "ent-001".to_string(),
            entity_type: "legal_entity".to_string(),
            display_name: "ООО Ромашка".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-01T10:00:00Z".to_string(),
            updated_at: "2026-08-03T12:00:00Z".to_string(),
        },
        "ent-002" => EntityMock {
            public_id: "ent-002".to_string(),
            entity_type: "person".to_string(),
            display_name: "Иван Иванов".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T10:00:00Z".to_string(),
            updated_at: "2026-08-03T11:00:00Z".to_string(),
        },
        _ => EntityMock {
            public_id: public_id.clone(),
            entity_type: "legal_entity".to_string(),
            display_name: "АО Вектор".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T14:00:00Z".to_string(),
            updated_at: "2026-08-03T10:00:00Z".to_string(),
        },
    };
    Json(ApiEnvelope { data: entity, meta: meta() })
}

// --- Schema drafts/published ---

#[derive(Debug, Serialize)]
pub struct SchemaDraftMock {
    pub revision_id: String,
    pub type_id: String,
    pub display_name: String,
    pub status: String,
    pub version: i32,
    pub created_at: String,
}

#[get("/schemas/drafts")]
pub async fn list_schema_drafts() -> Json<ApiEnvelope<Vec<SchemaDraftMock>>> {
    Json(ApiEnvelope {
        data: vec![],
        meta: meta(),
    })
}

#[get("/schemas/published")]
pub async fn list_schema_published() -> Json<ApiEnvelope<Vec<SchemaDraftMock>>> {
    let types = vec![
        SchemaDraftMock {
            revision_id: "rev-001".to_string(),
            type_id: "person".to_string(),
            display_name: "Физическое лицо".to_string(),
            status: "published".to_string(),
            version: 1,
            created_at: "2026-08-01T10:00:00Z".to_string(),
        },
        SchemaDraftMock {
            revision_id: "rev-002".to_string(),
            type_id: "legal_entity".to_string(),
            display_name: "Юридическое лицо".to_string(),
            status: "published".to_string(),
            version: 1,
            created_at: "2026-08-01T10:00:00Z".to_string(),
        },
        SchemaDraftMock {
            revision_id: "rev-003".to_string(),
            type_id: "request".to_string(),
            display_name: "Заявка".to_string(),
            status: "published".to_string(),
            version: 1,
            created_at: "2026-08-01T10:00:00Z".to_string(),
        },
    ];
    Json(ApiEnvelope { data: types, meta: meta() })
}

/// Route group.
pub fn routes() -> Vec<rocket::Route> {
    routes![
        query_audit,
        latest_checkpoint,
        create_checkpoint,
        verify_chain,
        get_retention,
        put_retention,
        query_checks,
        list_connectors,
        list_schema_types,
        query_entities,
        query_entities_post,
        list_schema_drafts,
        list_schema_published,
        get_entity,
    ]
}
