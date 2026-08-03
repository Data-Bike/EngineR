//! Mock data routes for Docker demo.
//!
//! Provides mock responses for audit, checks, integrations, entities,
//! schemas, connectors, files, and other endpoints that require a DB.
//!
//! In production these are backed by real repositories.

use rocket::serde::json::Json;
use rocket::State;
use rocket::{delete, get, post, put, routes};
use serde::Serialize;
use std::sync::Mutex;

/// API envelope wrapper.
#[derive(Debug, Serialize)]
pub struct ApiEnvelope<T: Serialize> {
    pub data: T,
    pub meta: serde_json::Value,
}

fn meta() -> serde_json::Value {
    serde_json::json!({ "request_id": "mock" })
}

// ---------------------------------------------------------------------------
// In-memory mock database (stateful so created items persist across requests)
// ---------------------------------------------------------------------------

/// In-memory store for mock data, wrapped in `Mutex` for thread-safe access
/// from Rocket handlers.
pub struct MockDb {
    pub drafts: Mutex<Vec<SchemaDraftMock>>,
    pub entities: Mutex<Vec<EntityMock>>,
    pub requests: Mutex<Vec<RequestMock>>,
    pub checks: Mutex<Vec<CheckInstanceMock>>,
    pub relations: Mutex<Vec<(String, RelationMock)>>, // (entity_id, relation)
    pub connectors: Mutex<Vec<ConnectorInstanceMock>>,
    pub reports: Mutex<Vec<ReportMock>>,
    pub tasks: Mutex<Vec<TaskMock>>,
    pub assessments: Mutex<Vec<(String, RiskAssessmentMock)>>, // (entity_id, assessment)
    pub documents: Mutex<Vec<(String, DocumentMock)>>, // (entity_id, doc)
    /// Monotonic counter for generating unique IDs.
    pub id_counter: Mutex<u64>,
}

impl MockDb {
    pub fn new() -> Self {
        Self {
            drafts: Mutex::new(vec![]),
            entities: Mutex::new(seed_entities()),
            requests: Mutex::new(seed_requests()),
            checks: Mutex::new(seed_checks()),
            relations: Mutex::new(seed_relations()),
            connectors: Mutex::new(seed_connectors()),
            reports: Mutex::new(seed_reports()),
            tasks: Mutex::new(seed_tasks()),
            assessments: Mutex::new(seed_assessments()),
            documents: Mutex::new(seed_documents()),
            id_counter: Mutex::new(100),
        }
    }

    /// Generate a unique numeric ID (monotonically increasing).
    pub fn next_id(&self) -> u64 {
        let mut c = self.id_counter.lock().unwrap();
        *c += 1;
        *c
    }
}

impl Default for MockDb {
    fn default() -> Self {
        Self::new()
    }
}

// --- Seed functions (return the same hardcoded data that was in the handlers) ---

fn seed_entities() -> Vec<EntityMock> {
    vec![
        EntityMock {
            public_id: "ent-001".to_string(),
            type_id: "legal_entity".to_string(),
            entity_class: "data".to_string(),
            revision: 1,
            display_name: "ООО Ромашка".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-01T10:00:00Z".to_string(),
            updated_at: "2026-08-03T12:00:00Z".to_string(),
            fields: Some(serde_json::json!({
                "full_name": "Общество с ограниченной ответственностью «Ромашка»",
                "short_name": "ООО Ромашка",
                "has_tax_id": "7701234567",
                "has_ogrn": "1027700123456",
                "kpp": "770101001",
                "opf": "ООО",
                "founded_date": "2020-01-15",
                "has_address": "Москва, ул. Тверская, 1",
                "managed_by": "Иван Иванов"
            })),
        },
        EntityMock {
            public_id: "ent-002".to_string(),
            type_id: "person".to_string(),
            entity_class: "data".to_string(),
            revision: 1,
            display_name: "Иван Иванов".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T10:00:00Z".to_string(),
            updated_at: "2026-08-03T11:00:00Z".to_string(),
            fields: Some(serde_json::json!({
                "full_name": "Иванов Иван Иванович",
                "date_of_birth": "1985-05-15",
                "inn": "770123456789",
                "snilz": "123-456-789 00",
                "citizenship": "Российская Федерация",
                "has_address": "Москва, ул. Ленина, 10"
            })),
        },
        EntityMock {
            public_id: "ent-003".to_string(),
            type_id: "legal_entity".to_string(),
            entity_class: "data".to_string(),
            revision: 1,
            display_name: "АО Вектор".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-02T14:00:00Z".to_string(),
            updated_at: "2026-08-03T10:00:00Z".to_string(),
            fields: Some(serde_json::json!({
                "full_name": "Акционерное общество «Вектор»",
                "short_name": "АО Вектор",
                "has_tax_id": "7709876543",
                "has_ogrn": "1037700987654",
                "kpp": "770901001",
                "opf": "АО",
                "founded_date": "2018-06-20",
                "has_address": "Санкт-Петербург, Невский пр., 100",
                "managed_by": "Петров Пётр Петрович"
            })),
        },
    ]
}

fn seed_requests() -> Vec<RequestMock> {
    vec![
        RequestMock {
            request_id: "req-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            initiator: "admin".to_string(),
            status: "in_progress".to_string(),
            requested_product: "Кредитный лимит".to_string(),
            requested_limit: Some(5000000.0),
            urgency: "normal".to_string(),
            due_date: Some("2026-08-10T00:00:00Z".to_string()),
            created_at: "2026-08-01T10:00:00Z".to_string(),
        },
        RequestMock {
            request_id: "req-002".to_string(),
            target_entity_id: "ent-002".to_string(),
            target_display_name: "Иван Иванов".to_string(),
            initiator: "admin".to_string(),
            status: "pending".to_string(),
            requested_product: "KYC".to_string(),
            requested_limit: None,
            urgency: "high".to_string(),
            due_date: Some("2026-08-07T00:00:00Z".to_string()),
            created_at: "2026-08-02T14:00:00Z".to_string(),
        },
        RequestMock {
            request_id: "req-003".to_string(),
            target_entity_id: "ent-003".to_string(),
            target_display_name: "АО Вектор".to_string(),
            initiator: "admin".to_string(),
            status: "completed".to_string(),
            requested_product: "Комплексная проверка".to_string(),
            requested_limit: Some(10000000.0),
            urgency: "normal".to_string(),
            due_date: Some("2026-08-05T00:00:00Z".to_string()),
            created_at: "2026-07-28T09:00:00Z".to_string(),
        },
    ]
}

fn seed_checks() -> Vec<CheckInstanceMock> {
    vec![
        CheckInstanceMock {
            check_id: "chk-001".to_string(),
            request_id: "req-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            kind: "sanction".to_string(),
            status: "completed".to_string(),
            started_at: "2026-08-03T10:00:00Z".to_string(),
            completed_at: Some("2026-08-03T10:05:00Z".to_string()),
            evidence: vec![],
            risk_level: Some("low".to_string()),
            step_up_required: false,
            data_denied: false,
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
            evidence: vec![],
            risk_level: None,
            step_up_required: false,
            data_denied: false,
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
            evidence: vec![],
            risk_level: None,
            step_up_required: false,
            data_denied: false,
        },
    ]
}

fn seed_relations() -> Vec<(String, RelationMock)> {
    vec![
        (
            "ent-001".to_string(),
            RelationMock {
                relation_id: "rel-001".to_string(),
                predicate: "managed_by".to_string(),
                direction: "outbound".to_string(),
                target_entity_id: "ent-002".to_string(),
                target_display_name: "Иван Иванов".to_string(),
                target_type: "person".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_to: None,
                source: "manual".to_string(),
                qualifiers: serde_json::json!({}),
            },
        ),
        (
            "ent-001".to_string(),
            RelationMock {
                relation_id: "rel-002".to_string(),
                predicate: "has_address".to_string(),
                direction: "outbound".to_string(),
                target_entity_id: "addr-001".to_string(),
                target_display_name: "Москва, ул. Тверская, 1".to_string(),
                target_type: "address".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_to: None,
                source: "manual".to_string(),
                qualifiers: serde_json::json!({ "kind": "legal" }),
            },
        ),
        (
            "ent-002".to_string(),
            RelationMock {
                relation_id: "rel-003".to_string(),
                predicate: "managed_by".to_string(),
                direction: "inbound".to_string(),
                target_entity_id: "ent-001".to_string(),
                target_display_name: "ООО Ромашка".to_string(),
                target_type: "legal_entity".to_string(),
                valid_from: "2026-01-01T00:00:00Z".to_string(),
                valid_to: None,
                source: "manual".to_string(),
                qualifiers: serde_json::json!({}),
            },
        ),
    ]
}

fn seed_connectors() -> Vec<ConnectorInstanceMock> {
    vec![
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
    ]
}

fn seed_reports() -> Vec<ReportMock> {
    vec![
        ReportMock {
            report_id: "rpt-001".to_string(),
            template: "individual_check".to_string(),
            format: "xlsx".to_string(),
            status: "completed".to_string(),
            created_at: "2026-08-03T09:00:00Z".to_string(),
            download_url: Some("/api/v1/files/rpt-001/download".to_string()),
        },
        ReportMock {
            report_id: "rpt-002".to_string(),
            template: "legal_entity_check".to_string(),
            format: "pdf".to_string(),
            status: "completed".to_string(),
            created_at: "2026-08-02T15:00:00Z".to_string(),
            download_url: Some("/api/v1/files/rpt-002/download".to_string()),
        },
        ReportMock {
            report_id: "rpt-003".to_string(),
            template: "statistics".to_string(),
            format: "xlsx".to_string(),
            status: "rendering".to_string(),
            created_at: "2026-08-03T14:00:00Z".to_string(),
            download_url: None,
        },
    ]
}

fn seed_tasks() -> Vec<TaskMock> {
    vec![
        TaskMock {
            task_id: "task-001".to_string(),
            kind: "review_check".to_string(),
            status: "pending".to_string(),
            priority: 10,
            assignee: Some("admin".to_string()),
            due_date: Some("2026-08-05T00:00:00Z".to_string()),
            created_at: "2026-08-03T10:00:00Z".to_string(),
            updated_at: "2026-08-03T10:00:00Z".to_string(),
            title: "Проверка санкционных списков".to_string(),
            sub_tasks: vec![],
            blocking: true,
        },
        TaskMock {
            task_id: "task-002".to_string(),
            kind: "collect_documents".to_string(),
            status: "in_progress".to_string(),
            priority: 5,
            assignee: Some("admin".to_string()),
            due_date: Some("2026-08-07T00:00:00Z".to_string()),
            created_at: "2026-08-03T11:00:00Z".to_string(),
            updated_at: "2026-08-03T12:00:00Z".to_string(),
            title: "Сбор финансовых документов".to_string(),
            sub_tasks: vec![],
            blocking: false,
        },
    ]
}

fn seed_assessments() -> Vec<(String, RiskAssessmentMock)> {
    vec![
        (
            "ent-001".to_string(),
            RiskAssessmentMock {
                assessment_id: "asmt-001".to_string(),
                target_entity_id: "ent-001".to_string(),
                target_display_name: "ООО Ромашка".to_string(),
                score: 35.0,
                risk_level: "low".to_string(),
                overall_level: "low".to_string(),
                decision: "accept".to_string(),
                status: "approved".to_string(),
                created_at: "2026-08-03T10:00:00Z".to_string(),
                assessor: "Система".to_string(),
                assessed_at: "2026-08-03T10:00:00Z".to_string(),
                rationale: None,
                approval_pending: false,
                factors: vec![
                    RiskFactorMock {
                        factor_id: "f-001".to_string(),
                        entity_public_id: "ent-001".to_string(),
                        criterion_id: "crit-001".to_string(),
                        label: "Срок регистрации".to_string(),
                        level: "info".to_string(),
                        resolved: false,
                        detected_at: "2026-08-03T10:00:00Z".to_string(),
                        resolved_at: None,
                        source_checks: vec!["chk-001".to_string()],
                        explanation: "Компания зарегистрирована более 3 лет назад".to_string(),
                    },
                ],
            },
        ),
        (
            "ent-003".to_string(),
            RiskAssessmentMock {
                assessment_id: "asmt-002".to_string(),
                target_entity_id: "ent-003".to_string(),
                target_display_name: "АО Вектор".to_string(),
                score: 72.0,
                risk_level: "high".to_string(),
                overall_level: "high".to_string(),
                decision: "escalate".to_string(),
                status: "pending_approval".to_string(),
                created_at: "2026-08-03T11:00:00Z".to_string(),
                assessor: "Система".to_string(),
                assessed_at: "2026-08-03T11:00:00Z".to_string(),
                rationale: Some("Требуется ручная проверка санкционного совпадения".to_string()),
                approval_pending: true,
                factors: vec![
                    RiskFactorMock {
                        factor_id: "f-002".to_string(),
                        entity_public_id: "ent-003".to_string(),
                        criterion_id: "crit-001".to_string(),
                        label: "Найден в санкционных списках".to_string(),
                        level: "critical".to_string(),
                        resolved: false,
                        detected_at: "2026-08-03T11:00:00Z".to_string(),
                        resolved_at: None,
                        source_checks: vec!["chk-001".to_string()],
                        explanation: "Совпадение по имени с записью в санкционном списке".to_string(),
                    },
                    RiskFactorMock {
                        factor_id: "f-003".to_string(),
                        entity_public_id: "ent-003".to_string(),
                        criterion_id: "crit-001".to_string(),
                        label: "Финансовое состояние".to_string(),
                        level: "medium".to_string(),
                        resolved: false,
                        detected_at: "2026-08-03T11:00:00Z".to_string(),
                        resolved_at: None,
                        source_checks: vec!["chk-001".to_string()],
                        explanation: "Отрицательная динамика за последние 6 месяцев".to_string(),
                    },
                ],
            },
        ),
        (
            "ent-002".to_string(),
            RiskAssessmentMock {
                assessment_id: "asmt-003".to_string(),
                target_entity_id: "ent-002".to_string(),
                target_display_name: "Иван Иванов".to_string(),
                score: 15.0,
                risk_level: "minimal".to_string(),
                overall_level: "minimal".to_string(),
                decision: "accept".to_string(),
                status: "approved".to_string(),
                created_at: "2026-08-02T16:00:00Z".to_string(),
                assessor: "Система".to_string(),
                assessed_at: "2026-08-02T16:00:00Z".to_string(),
                rationale: None,
                approval_pending: false,
                factors: vec![],
            },
        ),
    ]
}

fn seed_documents() -> Vec<(String, DocumentMock)> {
    vec![
        (
            "ent-001".to_string(),
            DocumentMock {
                document_id: "doc-001".to_string(),
                kind: "registration_certificate".to_string(),
                filename: "ОГРН_Ромашка.pdf".to_string(),
                size_bytes: 245678,
                content_type: "application/pdf".to_string(),
                uploaded_at: "2026-08-01T12:00:00Z".to_string(),
                valid_from: Some("2020-01-15T00:00:00Z".to_string()),
                valid_to: None,
            },
        ),
        (
            "ent-001".to_string(),
            DocumentMock {
                document_id: "doc-002".to_string(),
                kind: "financial_statement".to_string(),
                filename: "Бухгалтерский_баланс_2024.xlsx".to_string(),
                size_bytes: 1023456,
                content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
                uploaded_at: "2026-08-02T14:00:00Z".to_string(),
                valid_from: Some("2025-01-01T00:00:00Z".to_string()),
                valid_to: Some("2025-12-31T00:00:00Z".to_string()),
            },
        ),
    ]
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

#[post("/audit/verify")]
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

#[derive(Debug, Serialize, Clone)]
pub struct CheckInstanceMock {
    pub check_id: String,
    pub request_id: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub kind: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub evidence: Vec<serde_json::Value>,
    pub risk_level: Option<String>,
    pub step_up_required: bool,
    pub data_denied: bool,
}

#[get("/checks?<status>&<kind>&<limit>&<cursor>")]
pub async fn query_checks(
    db: &State<MockDb>,
    status: Option<String>,
    kind: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<CheckInstanceMock>>> {
    let _ = (limit, cursor);
    let checks: Vec<CheckInstanceMock> = db
        .checks
        .lock()
        .unwrap()
        .iter()
        .filter(|c| status.as_ref().map_or(true, |s| &c.status == s))
        .filter(|c| kind.as_ref().map_or(true, |k| &c.kind == k))
        .cloned()
        .collect();
    Json(ApiEnvelope { data: checks, meta: meta() })
}

// --- Connectors / Integrations ---

#[derive(Debug, Serialize, Clone)]
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
pub async fn list_connectors(db: &State<MockDb>) -> Json<ApiEnvelope<Vec<ConnectorInstanceMock>>> {
    let connectors = db.connectors.lock().unwrap().clone();
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

#[derive(Debug, Serialize, Clone)]
pub struct EntityMock {
    pub public_id: String,
    pub type_id: String,
    pub entity_class: String,
    pub revision: i64,
    pub display_name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<serde_json::Value>,
}

#[get("/entities?<type>&<limit>&<cursor>")]
pub async fn query_entities(
    db: &State<MockDb>,
    r#type: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<EntityMock>>> {
    let _ = (limit, cursor);
    let entities: Vec<EntityMock> = db
        .entities
        .lock()
        .unwrap()
        .iter()
        .filter(|e| r#type.as_ref().map_or(true, |t| &e.type_id == t))
        .cloned()
        .collect();
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
pub async fn query_entities_post(
    db: &State<MockDb>,
    body: String,
) -> Json<ApiEnvelope<QueryResponseMock>> {
    let parsed = serde_json::from_str::<serde_json::Value>(&body).ok();

    // Extract type_id from `from.type_id` for filtering
    let type_id = parsed
        .as_ref()
        .and_then(|v| {
            v.get("from")
                .and_then(|f| f.get("type_id"))
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
        .unwrap_or_default();

    // Extract optional search string
    let search = parsed
        .as_ref()
        .and_then(|v| v.get("search").and_then(|s| s.as_str()).map(|s| s.to_lowercase()))
        .unwrap_or_default();

    // Extract optional field-specific filters (e.g. inn, full_name)
    // Supports both array format [{field, value}] and object format {field: value}
    let field_filters: Vec<(String, String)> = parsed
        .as_ref()
        .and_then(|v| v.get("filters"))
        .map(|f| {
            let mut result = Vec::new();
            if let Some(arr) = f.as_array() {
                for item in arr {
                    if let (Some(field), Some(value)) = (
                        item.get("field").and_then(|f| f.as_str()),
                        item.get("value").and_then(|v| v.as_str()),
                    ) {
                        result.push((field.to_string(), value.to_lowercase()));
                    }
                }
            } else if let Some(obj) = f.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        result.push((k.clone(), s.to_lowercase()));
                    }
                }
            }
            result
        })
        .unwrap_or_default();

    let entities = db.entities.lock().unwrap();
    let rows: Vec<serde_json::Value> = entities
        .iter()
        .filter(|e| type_id.is_empty() || e.type_id == type_id)
        .filter(|e| {
            if search.is_empty() {
                return true;
            }
            // Search across display_name and all string fields
            if e.display_name.to_lowercase().contains(&search) {
                return true;
            }
            if let Some(fields) = &e.fields {
                if let Some(obj) = fields.as_object() {
                    for v in obj.values() {
                        if let Some(s) = v.as_str() {
                            if s.to_lowercase().contains(&search) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        })
        .filter(|e| {
            if field_filters.is_empty() {
                return true;
            }
            let fields = e.fields.as_ref().and_then(|f| f.as_object());
            for (field, value) in &field_filters {
                let matched = fields
                    .and_then(|obj| obj.get(field))
                    .and_then(|v| v.as_str())
                    .map_or(false, |s| s.to_lowercase().contains(value));
                if !matched {
                    return false;
                }
            }
            true
        })
        .map(|e| {
            let mut row = serde_json::json!({
                "public_id": e.public_id,
                "display_name": e.display_name,
                "type_id": e.type_id,
                "status": e.status,
            });
            if let Some(fields) = &e.fields {
                if let Some(obj) = row.as_object_mut() {
                    if let Some(fields_obj) = fields.as_object() {
                        for (k, v) in fields_obj {
                            obj.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
            row
        })
        .collect();

    let total = rows.len() as i64;
    Json(ApiEnvelope {
        data: QueryResponseMock {
            rows,
            cursor: None,
            total_estimate: Some(total),
        },
        meta: meta(),
    })
}

// --- Get single entity ---

#[get("/entities/<public_id>")]
pub async fn get_entity(
    db: &State<MockDb>,
    public_id: String,
) -> Json<ApiEnvelope<EntityMock>> {
    let entity = db
        .entities
        .lock()
        .unwrap()
        .iter()
        .find(|e| e.public_id == public_id)
        .cloned()
        .unwrap_or_else(|| EntityMock {
            public_id: public_id.clone(),
            type_id: "legal_entity".to_string(),
            entity_class: "data".to_string(),
            revision: 1,
            display_name: "Новая сущность".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-03T15:00:00Z".to_string(),
            updated_at: "2026-08-03T15:00:00Z".to_string(),
            fields: Some(serde_json::json!({})),
        });
    Json(ApiEnvelope { data: entity, meta: meta() })
}

// --- Schema drafts/published ---

#[derive(Debug, Serialize, Clone)]
pub struct SchemaDraftMock {
    pub revision_id: String,
    pub type_id: String,
    pub display_name: String,
    pub status: String,
    pub version: i32,
    pub created_at: String,
}

#[get("/schemas/drafts")]
pub async fn list_schema_drafts(db: &State<MockDb>) -> Json<ApiEnvelope<Vec<SchemaDraftMock>>> {
    let drafts = db.drafts.lock().unwrap().clone();
    Json(ApiEnvelope { data: drafts, meta: meta() })
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

// --- Requests ---

#[derive(Debug, Serialize, Clone)]
pub struct RequestMock {
    pub request_id: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub initiator: String,
    pub status: String,
    pub requested_product: String,
    pub requested_limit: Option<f64>,
    pub urgency: String,
    pub due_date: Option<String>,
    pub created_at: String,
}

#[get("/requests?<status>&<limit>&<cursor>")]
pub async fn query_requests(
    db: &State<MockDb>,
    status: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<RequestMock>>> {
    let _ = (limit, cursor);
    let requests: Vec<RequestMock> = db
        .requests
        .lock()
        .unwrap()
        .iter()
        .filter(|r| status.as_ref().map_or(true, |s| &r.status == s))
        .cloned()
        .collect();
    Json(ApiEnvelope { data: requests, meta: meta() })
}

#[post("/requests", data = "<body>")]
pub async fn create_request(body: String) -> Json<ApiEnvelope<RequestMock>> {
    let _ = body;
    Json(ApiEnvelope {
        data: RequestMock {
            request_id: "req-004".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            initiator: "admin".to_string(),
            status: "pending".to_string(),
            requested_product: "KYC".to_string(),
            requested_limit: None,
            urgency: "normal".to_string(),
            due_date: None,
            created_at: "2026-08-03T15:00:00Z".to_string(),
        },
        meta: meta(),
    })
}

// --- Risk Assessments ---

#[derive(Debug, Serialize, Clone)]
pub struct RiskAssessmentMock {
    pub assessment_id: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub score: f64,
    pub risk_level: String,
    pub overall_level: String,
    pub decision: String,
    pub status: String,
    pub created_at: String,
    pub assessor: String,
    pub assessed_at: String,
    pub rationale: Option<String>,
    pub approval_pending: bool,
    pub factors: Vec<RiskFactorMock>,
}

#[derive(Debug, Serialize, Clone)]
pub struct RiskFactorMock {
    pub factor_id: String,
    pub entity_public_id: String,
    pub criterion_id: String,
    pub label: String,
    pub level: String,
    pub resolved: bool,
    pub detected_at: String,
    pub resolved_at: Option<String>,
    pub source_checks: Vec<String>,
    pub explanation: String,
}

#[get("/assessments?<status>&<limit>&<cursor>")]
pub async fn query_assessments(
    status: Option<String>,
    limit: Option<u32>,
    cursor: Option<String>,
) -> Json<ApiEnvelope<Vec<RiskAssessmentMock>>> {
    let _ = (status, limit, cursor);
    let assessments = vec![
        RiskAssessmentMock {
            assessment_id: "asmt-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            score: 35.0,
            risk_level: "low".to_string(),
            overall_level: "low".to_string(),
            decision: "accept".to_string(),
            status: "approved".to_string(),
            created_at: "2026-08-03T10:00:00Z".to_string(),
            assessor: "Система".to_string(),
            assessed_at: "2026-08-03T10:00:00Z".to_string(),
            rationale: None,
            approval_pending: false,
            factors: vec![
                RiskFactorMock {
                    factor_id: "f-001".to_string(),
                    entity_public_id: "ent-001".to_string(),
                    criterion_id: "crit-001".to_string(),
                    label: "Срок регистрации".to_string(),
                    level: "info".to_string(),
                    resolved: false,
                    detected_at: "2026-08-03T10:00:00Z".to_string(),
                    resolved_at: None,
                    source_checks: vec!["chk-001".to_string()],
                    explanation: "Компания зарегистрирована более 3 лет назад".to_string(),
                },
            ],
        },
        RiskAssessmentMock {
            assessment_id: "asmt-002".to_string(),
            target_entity_id: "ent-003".to_string(),
            target_display_name: "АО Вектор".to_string(),
            score: 72.0,
            risk_level: "high".to_string(),
            overall_level: "high".to_string(),
            decision: "escalate".to_string(),
            status: "pending_approval".to_string(),
            created_at: "2026-08-03T11:00:00Z".to_string(),
            assessor: "Система".to_string(),
            assessed_at: "2026-08-03T11:00:00Z".to_string(),
            rationale: Some("Требуется ручная проверка санкционного совпадения".to_string()),
            approval_pending: true,
            factors: vec![
                RiskFactorMock {
                    factor_id: "f-002".to_string(),
                    entity_public_id: "ent-003".to_string(),
                    criterion_id: "crit-001".to_string(),
                    label: "Найден в санкционных списках".to_string(),
                    level: "critical".to_string(),
                    resolved: false,
                    detected_at: "2026-08-03T11:00:00Z".to_string(),
                    resolved_at: None,
                    source_checks: vec!["chk-001".to_string()],
                    explanation: "Совпадение по имени с записью в санкционном списке".to_string(),
                },
                RiskFactorMock {
                    factor_id: "f-003".to_string(),
                    entity_public_id: "ent-003".to_string(),
                    criterion_id: "crit-001".to_string(),
                    label: "Финансовое состояние".to_string(),
                    level: "medium".to_string(),
                    resolved: false,
                    detected_at: "2026-08-03T11:00:00Z".to_string(),
                    resolved_at: None,
                    source_checks: vec!["chk-001".to_string()],
                    explanation: "Отрицательная динамика за последние 6 месяцев".to_string(),
                },
            ],
        },
        RiskAssessmentMock {
            assessment_id: "asmt-003".to_string(),
            target_entity_id: "ent-002".to_string(),
            target_display_name: "Иван Иванов".to_string(),
            score: 15.0,
            risk_level: "minimal".to_string(),
            overall_level: "minimal".to_string(),
            decision: "accept".to_string(),
            status: "approved".to_string(),
            created_at: "2026-08-02T16:00:00Z".to_string(),
            assessor: "Система".to_string(),
            assessed_at: "2026-08-02T16:00:00Z".to_string(),
            rationale: None,
            approval_pending: false,
            factors: vec![],
        },
    ];
    Json(ApiEnvelope { data: assessments, meta: meta() })
}

// --- Monitors ---

#[derive(Debug, Serialize)]
pub struct MonitorMock {
    pub monitor_id: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub frequency: String,
    pub next_run: String,
    pub last_result: Option<String>,
    pub status: String,
}

#[get("/monitors")]
pub async fn list_monitors() -> Json<ApiEnvelope<Vec<MonitorMock>>> {
    let monitors = vec![
        MonitorMock {
            monitor_id: "mon-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            frequency: "monthly".to_string(),
            next_run: "2026-09-01T00:00:00Z".to_string(),
            last_result: Some("ok".to_string()),
            status: "active".to_string(),
        },
        MonitorMock {
            monitor_id: "mon-002".to_string(),
            target_entity_id: "ent-003".to_string(),
            target_display_name: "АО Вектор".to_string(),
            frequency: "weekly".to_string(),
            next_run: "2026-08-07T00:00:00Z".to_string(),
            last_result: Some("changes_detected".to_string()),
            status: "active".to_string(),
        },
        MonitorMock {
            monitor_id: "mon-003".to_string(),
            target_entity_id: "ent-002".to_string(),
            target_display_name: "Иван Иванов".to_string(),
            frequency: "quarterly".to_string(),
            next_run: "2026-10-01T00:00:00Z".to_string(),
            last_result: None,
            status: "paused".to_string(),
        },
    ];
    Json(ApiEnvelope { data: monitors, meta: meta() })
}

// --- Reports ---

#[derive(Debug, Serialize, Clone)]
pub struct ReportMock {
    pub report_id: String,
    pub template: String,
    pub format: String,
    pub status: String,
    pub created_at: String,
    pub download_url: Option<String>,
}

#[get("/reports")]
pub async fn list_reports() -> Json<ApiEnvelope<Vec<ReportMock>>> {
    let reports = vec![
        ReportMock {
            report_id: "rpt-001".to_string(),
            template: "individual_check".to_string(),
            format: "xlsx".to_string(),
            status: "completed".to_string(),
            created_at: "2026-08-03T09:00:00Z".to_string(),
            download_url: Some("/api/v1/files/rpt-001/download".to_string()),
        },
        ReportMock {
            report_id: "rpt-002".to_string(),
            template: "legal_entity_check".to_string(),
            format: "pdf".to_string(),
            status: "completed".to_string(),
            created_at: "2026-08-02T15:00:00Z".to_string(),
            download_url: Some("/api/v1/files/rpt-002/download".to_string()),
        },
        ReportMock {
            report_id: "rpt-003".to_string(),
            template: "statistics".to_string(),
            format: "xlsx".to_string(),
            status: "rendering".to_string(),
            created_at: "2026-08-03T14:00:00Z".to_string(),
            download_url: None,
        },
    ];
    Json(ApiEnvelope { data: reports, meta: meta() })
}

#[post("/reports", data = "<body>")]
pub async fn create_report(body: String) -> Json<ApiEnvelope<ReportMock>> {
    let _ = body;
    Json(ApiEnvelope {
        data: ReportMock {
            report_id: "rpt-004".to_string(),
            template: "individual_check".to_string(),
            format: "xlsx".to_string(),
            status: "pending".to_string(),
            created_at: "2026-08-03T15:00:00Z".to_string(),
            download_url: None,
        },
        meta: meta(),
    })
}

// --- Dashboard stats ---

#[derive(Debug, Serialize)]
pub struct DashboardStatsMock {
    pub my_tasks: i64,
    pub overdue_tasks: i64,
    pub new_requests: i64,
    pub checks_in_progress: i64,
    pub checks_errors: i64,
    pub critical_factors: i64,
    pub monitors_due: i64,
    pub ready_reports: i64,
}

#[get("/dashboard/stats")]
pub async fn dashboard_stats() -> Json<ApiEnvelope<DashboardStatsMock>> {
    Json(ApiEnvelope {
        data: DashboardStatsMock {
            my_tasks: 5,
            overdue_tasks: 1,
            new_requests: 3,
            checks_in_progress: 2,
            checks_errors: 0,
            critical_factors: 1,
            monitors_due: 4,
            ready_reports: 2,
        },
        meta: meta(),
    })
}

// --- Entity detail: relations ---

#[derive(Debug, Serialize, Clone)]
pub struct RelationMock {
    pub relation_id: String,
    pub predicate: String,
    pub direction: String,
    pub target_entity_id: String,
    pub target_display_name: String,
    pub target_type: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub source: String,
    pub qualifiers: serde_json::Value,
}

#[get("/entities/<public_id>/relations?<direction>&<as_of>")]
pub async fn get_relations(
    db: &State<MockDb>,
    public_id: String,
    direction: Option<String>,
    as_of: Option<String>,
) -> Json<ApiEnvelope<Vec<RelationMock>>> {
    let _ = (direction, as_of);
    let relations = db.relations.lock().unwrap()
        .iter()
        .filter(|(eid, _)| eid == &public_id)
        .map(|(_, r)| r.clone())
        .collect();
    Json(ApiEnvelope { data: relations, meta: meta() })
}

#[post("/entities/<public_id>/relations", data = "<body>")]
pub async fn create_relation(
    db: &State<MockDb>,
    public_id: String,
    body: String,
) -> Json<ApiEnvelope<RelationMock>> {
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let rel = RelationMock {
        relation_id: format!("rel-{}", db.next_id()),
        predicate: parsed.get("predicate").and_then(|v| v.as_str()).unwrap_or("managed_by").to_string(),
        direction: parsed.get("direction").and_then(|v| v.as_str()).unwrap_or("outgoing").to_string(),
        target_entity_id: parsed.get("target_entity_id").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        target_display_name: parsed.get("target_display_name").and_then(|v| v.as_str()).unwrap_or("Новый объект").to_string(),
        target_type: parsed.get("target_type").and_then(|v| v.as_str()).unwrap_or("person").to_string(),
        valid_from: parsed.get("valid_from").and_then(|v| v.as_str()).unwrap_or("2026-01-01T00:00:00Z").to_string(),
        valid_to: parsed.get("valid_to").and_then(|v| v.as_str()).map(|s| s.to_string()),
        source: parsed.get("source").and_then(|v| v.as_str()).unwrap_or("manual").to_string(),
        qualifiers: parsed.get("qualifiers").cloned().unwrap_or(serde_json::json!({})),
    };
    db.relations.lock().unwrap().push((public_id, rel.clone()));
    Json(ApiEnvelope { data: rel, meta: meta() })
}

#[delete("/entities/<public_id>/relations/<relation_id>")]
pub async fn delete_relation(
    db: &State<MockDb>,
    public_id: String,
    relation_id: String,
) -> Json<ApiEnvelope<serde_json::Value>> {
    let mut rels = db.relations.lock().unwrap();
    rels.retain(|(eid, r)| !(eid == &public_id && r.relation_id == relation_id));
    Json(ApiEnvelope {
        data: serde_json::json!({ "deleted": true }),
        meta: meta(),
    })
}

// --- Entity detail: checks ---

#[get("/entities/<public_id>/checks")]
pub async fn get_entity_checks(public_id: String) -> Json<ApiEnvelope<Vec<CheckInstanceMock>>> {
    let checks = match public_id.as_str() {
        "ent-001" => vec![
            CheckInstanceMock {
                check_id: "chk-001".to_string(),
                request_id: "req-001".to_string(),
                target_entity_id: "ent-001".to_string(),
                target_display_name: "ООО Ромашка".to_string(),
                kind: "sanction".to_string(),
                status: "completed".to_string(),
                started_at: "2026-08-03T10:00:00Z".to_string(),
                completed_at: Some("2026-08-03T10:05:00Z".to_string()),
                evidence: vec![],
                risk_level: Some("low".to_string()),
                step_up_required: false,
                data_denied: false,
            },
        ],
        "ent-002" => vec![
            CheckInstanceMock {
                check_id: "chk-002".to_string(),
                request_id: "req-002".to_string(),
                target_entity_id: "ent-002".to_string(),
                target_display_name: "Иван Иванов".to_string(),
                kind: "kyc".to_string(),
                status: "running".to_string(),
                started_at: "2026-08-03T11:00:00Z".to_string(),
                completed_at: None,
                evidence: vec![],
                risk_level: None,
                step_up_required: false,
                data_denied: false,
            },
        ],
        _ => vec![],
    };
    Json(ApiEnvelope { data: checks, meta: meta() })
}

// --- Entity detail: risks/assessment ---

#[get("/entities/<public_id>/assessment")]
pub async fn get_entity_assessment(public_id: String) -> Json<ApiEnvelope<Option<RiskAssessmentMock>>> {
    let assessment = match public_id.as_str() {
        "ent-001" => Some(RiskAssessmentMock {
            assessment_id: "asmt-001".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            score: 35.0,
            risk_level: "low".to_string(),
            overall_level: "low".to_string(),
            decision: "accept".to_string(),
            status: "approved".to_string(),
            created_at: "2026-08-03T10:00:00Z".to_string(),
            assessor: "Система".to_string(),
            assessed_at: "2026-08-03T10:00:00Z".to_string(),
            rationale: None,
            approval_pending: false,
            factors: vec![
                RiskFactorMock {
                    factor_id: "f-001".to_string(),
                    entity_public_id: "ent-001".to_string(),
                    criterion_id: "crit-001".to_string(),
                    label: "Срок регистрации".to_string(),
                    level: "info".to_string(),
                    resolved: false,
                    detected_at: "2026-08-03T10:00:00Z".to_string(),
                    resolved_at: None,
                    source_checks: vec!["chk-001".to_string()],
                    explanation: "Компания зарегистрирована более 3 лет назад".to_string(),
                },
            ],
        }),
        "ent-003" => Some(RiskAssessmentMock {
            assessment_id: "asmt-002".to_string(),
            target_entity_id: "ent-003".to_string(),
            target_display_name: "АО Вектор".to_string(),
            score: 72.0,
            risk_level: "high".to_string(),
            overall_level: "high".to_string(),
            decision: "escalate".to_string(),
            status: "pending_approval".to_string(),
            created_at: "2026-08-03T11:00:00Z".to_string(),
            assessor: "Система".to_string(),
            assessed_at: "2026-08-03T11:00:00Z".to_string(),
            rationale: Some("Требуется ручная проверка санкционного совпадения".to_string()),
            approval_pending: true,
            factors: vec![
                RiskFactorMock {
                    factor_id: "f-002".to_string(),
                    entity_public_id: "ent-003".to_string(),
                    criterion_id: "crit-001".to_string(),
                    label: "Санкционный список".to_string(),
                    level: "critical".to_string(),
                    resolved: false,
                    detected_at: "2026-08-03T11:00:00Z".to_string(),
                    resolved_at: None,
                    source_checks: vec!["chk-001".to_string()],
                    explanation: "Совпадение по имени с записью в санкционном списке".to_string(),
                },
            ],
        }),
        _ => None,
    };
    Json(ApiEnvelope { data: assessment, meta: meta() })
}

// --- Entity detail: tasks ---

#[derive(Debug, Serialize, Clone)]
pub struct TaskMock {
    pub task_id: String,
    pub kind: String,
    pub status: String,
    pub priority: i32,
    pub assignee: Option<String>,
    pub due_date: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub sub_tasks: Vec<String>,
    pub blocking: bool,
}

#[get("/entities/<public_id>/tasks")]
pub async fn get_entity_tasks(public_id: String) -> Json<ApiEnvelope<Vec<TaskMock>>> {
    let _ = public_id;
    let tasks = vec![
        TaskMock {
            task_id: "task-001".to_string(),
            kind: "review_check".to_string(),
            status: "pending".to_string(),
            priority: 10,
            assignee: Some("admin".to_string()),
            due_date: Some("2026-08-05T00:00:00Z".to_string()),
            created_at: "2026-08-03T10:00:00Z".to_string(),
            updated_at: "2026-08-03T10:00:00Z".to_string(),
            title: "Проверка санкционных списков".to_string(),
            sub_tasks: vec![],
            blocking: true,
        },
        TaskMock {
            task_id: "task-002".to_string(),
            kind: "collect_documents".to_string(),
            status: "in_progress".to_string(),
            priority: 5,
            assignee: Some("admin".to_string()),
            due_date: Some("2026-08-07T00:00:00Z".to_string()),
            created_at: "2026-08-03T11:00:00Z".to_string(),
            updated_at: "2026-08-03T12:00:00Z".to_string(),
            title: "Сбор финансовых документов".to_string(),
            sub_tasks: vec![],
            blocking: false,
        },
    ];
    Json(ApiEnvelope { data: tasks, meta: meta() })
}

// --- Entity detail: documents ---

#[derive(Debug, Serialize, Clone)]
pub struct DocumentMock {
    pub document_id: String,
    pub kind: String,
    pub filename: String,
    pub size_bytes: i64,
    pub content_type: String,
    pub uploaded_at: String,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

#[get("/entities/<public_id>/documents")]
pub async fn get_entity_documents(public_id: String) -> Json<ApiEnvelope<Vec<DocumentMock>>> {
    let _ = public_id;
    let docs = vec![
        DocumentMock {
            document_id: "doc-001".to_string(),
            kind: "registration_certificate".to_string(),
            filename: "ОГРН_Ромашка.pdf".to_string(),
            size_bytes: 245678,
            content_type: "application/pdf".to_string(),
            uploaded_at: "2026-08-01T12:00:00Z".to_string(),
            valid_from: Some("2020-01-15T00:00:00Z".to_string()),
            valid_to: None,
        },
        DocumentMock {
            document_id: "doc-002".to_string(),
            kind: "financial_statement".to_string(),
            filename: "Бухгалтерский_отчет_2025.xlsx".to_string(),
            size_bytes: 1023456,
            content_type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet".to_string(),
            uploaded_at: "2026-08-02T14:00:00Z".to_string(),
            valid_from: Some("2025-01-01T00:00:00Z".to_string()),
            valid_to: Some("2025-12-31T00:00:00Z".to_string()),
        },
    ];
    Json(ApiEnvelope { data: docs, meta: meta() })
}

// --- Entity detail: history ---

#[derive(Debug, Serialize)]
pub struct HistoryEntryMock {
    pub revision: i64,
    pub timestamp: String,
    pub actor: String,
    pub reason: String,
    pub changes: Vec<HistoryChangeMock>,
}

#[derive(Debug, Serialize)]
pub struct HistoryChangeMock {
    pub field: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[get("/entities/<public_id>/history")]
pub async fn get_entity_history(public_id: String) -> Json<ApiEnvelope<Vec<HistoryEntryMock>>> {
    let _ = public_id;
    let history = vec![
        HistoryEntryMock {
            revision: 1,
            timestamp: "2026-08-01T10:00:00Z".to_string(),
            actor: "admin".to_string(),
            reason: "Создание сущности".to_string(),
            changes: vec![],
        },
        HistoryEntryMock {
            revision: 2,
            timestamp: "2026-08-02T14:00:00Z".to_string(),
            actor: "admin".to_string(),
            reason: "Добавление финансового документа".to_string(),
            changes: vec![
                HistoryChangeMock {
                    field: "documents".to_string(),
                    old_value: None,
                    new_value: Some("doc-002".to_string()),
                },
            ],
        },
        HistoryEntryMock {
            revision: 3,
            timestamp: "2026-08-03T12:00:00Z".to_string(),
            actor: "admin".to_string(),
            reason: "Обновление адреса".to_string(),
            changes: vec![
                HistoryChangeMock {
                    field: "has_address".to_string(),
                    old_value: Some("Москва, ул. Ленина, 10".to_string()),
                    new_value: Some("Москва, ул. Тверская, 1".to_string()),
                },
            ],
        },
    ];
    Json(ApiEnvelope { data: history, meta: meta() })
}

// --- Entity detail: graph ---

#[derive(Debug, Serialize)]
pub struct GraphNodeMock {
    pub id: String,
    pub label: String,
    pub entity_type: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct GraphEdgeMock {
    pub source: String,
    pub target: String,
    pub predicate: String,
    pub label: String,
}

#[derive(Debug, Serialize)]
pub struct GraphDataMock {
    pub nodes: Vec<GraphNodeMock>,
    pub edges: Vec<GraphEdgeMock>,
}

#[post("/entities/<public_id>/graph", data = "<body>")]
pub async fn get_entity_graph(public_id: String, body: String) -> Json<ApiEnvelope<GraphDataMock>> {
    let _ = body;
    let (nodes, edges) = match public_id.as_str() {
        "ent-001" => (
            vec![
                GraphNodeMock { id: "ent-001".to_string(), label: "ООО Ромашка".to_string(), entity_type: "legal_entity".to_string(), status: "active".to_string() },
                GraphNodeMock { id: "ent-002".to_string(), label: "Иван Иванов".to_string(), entity_type: "person".to_string(), status: "active".to_string() },
                GraphNodeMock { id: "addr-001".to_string(), label: "Москва, ул. Тверская, 1".to_string(), entity_type: "address".to_string(), status: "active".to_string() },
            ],
            vec![
                GraphEdgeMock { source: "ent-001".to_string(), target: "ent-002".to_string(), predicate: "managed_by".to_string(), label: "управляет".to_string() },
                GraphEdgeMock { source: "ent-001".to_string(), target: "addr-001".to_string(), predicate: "has_address".to_string(), label: "адрес".to_string() },
            ],
        ),
        _ => (vec![], vec![]),
    };
    Json(ApiEnvelope { data: GraphDataMock { nodes, edges }, meta: meta() })
}

// --- Schema predicates ---

#[derive(Debug, Serialize)]
pub struct PredicateMock {
    pub predicate_id: String,
    pub alias: String,
    pub display_name: String,
    pub domain: String,
    pub range: String,
    pub cardinality_min: i32,
    pub cardinality_max: Option<i32>,
    pub required: bool,
}

#[get("/schemas/predicates")]
pub async fn list_predicates() -> Json<ApiEnvelope<Vec<PredicateMock>>> {
    let predicates = vec![
        PredicateMock { predicate_id: "pred-001".to_string(), alias: "has_tax_id".to_string(), display_name: "ИНН".to_string(), domain: "legal_entity".to_string(), range: "text".to_string(), cardinality_min: 0, cardinality_max: Some(1), required: false },
        PredicateMock { predicate_id: "pred-002".to_string(), alias: "has_ogrn".to_string(), display_name: "ОГРН".to_string(), domain: "legal_entity".to_string(), range: "text".to_string(), cardinality_min: 0, cardinality_max: Some(1), required: false },
        PredicateMock { predicate_id: "pred-003".to_string(), alias: "managed_by".to_string(), display_name: "Управляет".to_string(), domain: "legal_entity".to_string(), range: "person".to_string(), cardinality_min: 0, cardinality_max: None, required: false },
        PredicateMock { predicate_id: "pred-004".to_string(), alias: "has_address".to_string(), display_name: "Адрес".to_string(), domain: "legal_entity".to_string(), range: "address".to_string(), cardinality_min: 0, cardinality_max: None, required: false },
        PredicateMock { predicate_id: "pred-005".to_string(), alias: "full_name".to_string(), display_name: "Полное наименование".to_string(), domain: "legal_entity".to_string(), range: "text".to_string(), cardinality_min: 1, cardinality_max: Some(1), required: true },
        PredicateMock { predicate_id: "pred-006".to_string(), alias: "full_name".to_string(), display_name: "ФИО".to_string(), domain: "person".to_string(), range: "text".to_string(), cardinality_min: 1, cardinality_max: Some(1), required: true },
        PredicateMock { predicate_id: "pred-007".to_string(), alias: "date_of_birth".to_string(), display_name: "Дата рождения".to_string(), domain: "person".to_string(), range: "date".to_string(), cardinality_min: 0, cardinality_max: Some(1), required: false },
        PredicateMock { predicate_id: "pred-008".to_string(), alias: "inn".to_string(), display_name: "ИНН".to_string(), domain: "person".to_string(), range: "text".to_string(), cardinality_min: 0, cardinality_max: Some(1), required: false },
    ];
    Json(ApiEnvelope { data: predicates, meta: meta() })
}

// --- Entity create ---

#[post("/entities", data = "<body>")]
pub async fn create_entity(body: String) -> Json<ApiEnvelope<EntityMock>> {
    let _ = body;
    Json(ApiEnvelope {
        data: EntityMock {
            public_id: "ent-004".to_string(),
            type_id: "person".to_string(),
            entity_class: "data".to_string(),
            revision: 1,
            display_name: "Новая сущность".to_string(),
            status: "active".to_string(),
            created_at: "2026-08-03T15:00:00Z".to_string(),
            updated_at: "2026-08-03T15:00:00Z".to_string(),
            fields: Some(serde_json::json!({})),
        },
        meta: meta(),
    })
}

// --- Connector actions ---

#[post("/connectors/<id>/health")]
pub async fn connector_health(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "instance_id": id, "status": "healthy", "latency_ms": 42 }),
        meta: meta(),
    })
}

#[post("/connectors/<id>/enable")]
pub async fn connector_enable(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "instance_id": id, "enabled": true }),
        meta: meta(),
    })
}

#[post("/connectors/<id>/disable")]
pub async fn connector_disable(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "instance_id": id, "enabled": false }),
        meta: meta(),
    })
}

#[post("/connectors", data = "<body>")]
pub async fn create_connector(body: String) -> Json<ApiEnvelope<ConnectorInstanceMock>> {
    let _ = body;
    Json(ApiEnvelope {
        data: ConnectorInstanceMock {
            instance_id: "conn-004".to_string(),
            connector_type: "api".to_string(),
            display_name: "Новый коннектор".to_string(),
            enabled: true,
            trust_rank: "Medium".to_string(),
            last_health_check: None,
            status: "healthy".to_string(),
        },
        meta: meta(),
    })
}

// --- Check actions ---

#[post("/checks", data = "<body>")]
pub async fn create_check(body: String) -> Json<ApiEnvelope<CheckInstanceMock>> {
    let _ = body;
    Json(ApiEnvelope {
        data: CheckInstanceMock {
            check_id: "chk-004".to_string(),
            request_id: "req-004".to_string(),
            target_entity_id: "ent-001".to_string(),
            target_display_name: "ООО Ромашка".to_string(),
            kind: "kyc".to_string(),
            status: "pending".to_string(),
            started_at: "2026-08-03T15:00:00Z".to_string(),
            completed_at: None,
            evidence: vec![],
            risk_level: None,
            step_up_required: false,
            data_denied: false,
        },
        meta: meta(),
    })
}

// --- Monitor run ---

#[post("/monitors/<id>/run")]
pub async fn run_monitor(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "monitor_id": id, "status": "started", "job_id": "job-001" }),
        meta: meta(),
    })
}

// --- Entity retire ---

#[derive(Debug, serde::Deserialize)]
pub struct RetireBody {
    pub reason: Option<String>,
}

#[post("/entities/<public_id>/retire", data = "<body>")]
pub async fn retire_entity(public_id: String, body: String) -> Json<ApiEnvelope<serde_json::Value>> {
    let pid = public_id.clone();
    let _ = body;
    Json(ApiEnvelope {
        data: serde_json::json!({ "public_id": pid, "status": "retired" }),
        meta: meta(),
    })
}

// --- Schema actions ---

#[post("/schemas/drafts", data = "<body>")]
pub async fn create_schema_draft(db: &State<MockDb>, body: String) -> Json<ApiEnvelope<SchemaDraftMock>> {
    let parsed: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
    let name = parsed.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Новый черновик")
        .to_string();
    let id = format!("rev-{}", db.next_id());
    let draft = SchemaDraftMock {
        revision_id: id.clone(),
        type_id: "custom".to_string(),
        display_name: name,
        status: "draft".to_string(),
        version: 1,
        created_at: "2026-08-03T15:00:00Z".to_string(),
    };
    db.drafts.lock().unwrap().push(draft.clone());
    Json(ApiEnvelope {
        data: draft,
        meta: meta(),
    })
}

#[post("/schemas/drafts/<id>/validate")]
pub async fn validate_schema_draft(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "revision_id": id, "valid": true, "warnings": [] }),
        meta: meta(),
    })
}

#[post("/schemas/drafts/<id>/publish")]
pub async fn publish_schema_draft(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "revision_id": id, "status": "published" }),
        meta: meta(),
    })
}

#[post("/schemas/drafts/<id>/impact")]
pub async fn impact_schema_draft(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "revision_id": id, "affected_entities": 0, "affected_statements": 0, "breaking": false }),
        meta: meta(),
    })
}

#[post("/schemas/drafts/<id>/retire")]
pub async fn retire_schema_draft(id: String) -> Json<ApiEnvelope<serde_json::Value>> {
    Json(ApiEnvelope {
        data: serde_json::json!({ "revision_id": id, "status": "retired" }),
        meta: meta(),
    })
}

// --- View definitions for entity forms ---

#[get("/schemas/types/<type_id>/view?<view_type>")]
pub async fn get_type_view(type_id: String, view_type: Option<String>) -> Json<ApiEnvelope<serde_json::Value>> {
    let vtype = view_type.unwrap_or_else(|| "form".to_string());
    let view = match type_id.as_str() {
        "person" => serde_json::json!({
            "view_id": "person_form",
            "view_type": vtype,
            "type_id": "person",
            "schema_revision": 1,
            "name_key": "types.person",
            "sections": [
                {
                    "label_key": "sections.main",
                    "order": 0,
                    "collapsible": false,
                    "default_collapsed": false,
                    "fields": [
                        { "predicate_id": "full_name", "alias": "full_name", "label_key": "fields.person.full_name", "kind": "text", "required": true, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 },
                        { "predicate_id": "date_of_birth", "alias": "date_of_birth", "label_key": "fields.person.date_of_birth", "kind": "date", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 1 },
                        { "predicate_id": "inn", "alias": "inn", "label_key": "fields.person.inn", "kind": "text", "required": false, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 2 },
                        { "predicate_id": "snilz", "alias": "snilz", "label_key": "fields.person.snils", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 3 },
                        { "predicate_id": "citizenship", "alias": "citizenship", "label_key": "fields.person.citizenship", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 4 },
                        { "predicate_id": "has_address", "alias": "has_address", "label_key": "fields.person.address", "kind": "entity_reference", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 5, "target_type_id": "address" }
                    ]
                }
            ],
            "columns": [
                { "predicate_id": "full_name", "alias": "full_name", "label_key": "fields.person.full_name", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 },
                { "predicate_id": "inn", "alias": "inn", "label_key": "fields.person.inn", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 1 },
                { "predicate_id": "date_of_birth", "alias": "date_of_birth", "label_key": "fields.person.date_of_birth", "kind": "date", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 2 }
            ],
            "default_sort": [{ "field": "full_name", "direction": "asc" }],
            "filters": [],
            "facets": [],
            "row_actions": [],
            "bulk_actions": [],
            "export_allowed": true
        }),
        "legal_entity" => serde_json::json!({
            "view_id": "legal_entity_form",
            "view_type": vtype,
            "type_id": "legal_entity",
            "schema_revision": 1,
            "name_key": "types.legal_entity",
            "sections": [
                {
                    "label_key": "sections.main",
                    "order": 0,
                    "collapsible": false,
                    "default_collapsed": false,
                    "fields": [
                        { "predicate_id": "full_name", "alias": "full_name", "label_key": "fields.legal_entity.full_name", "kind": "text", "required": true, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 },
                        { "predicate_id": "short_name", "alias": "short_name", "label_key": "fields.legal_entity.short_name", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 1 },
                        { "predicate_id": "has_tax_id", "alias": "has_tax_id", "label_key": "fields.legal_entity.inn", "kind": "text", "required": false, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 2 },
                        { "predicate_id": "has_ogrn", "alias": "has_ogrn", "label_key": "fields.legal_entity.ogrn", "kind": "text", "required": false, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 3 },
                        { "predicate_id": "kpp", "alias": "kpp", "label_key": "fields.legal_entity.kpp", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 4 },
                        { "predicate_id": "opf", "alias": "opf", "label_key": "fields.legal_entity.opf", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 5 },
                        { "predicate_id": "founded_date", "alias": "founded_date", "label_key": "fields.legal_entity.founded_date", "kind": "date", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 6 },
                        { "predicate_id": "has_address", "alias": "has_address", "label_key": "fields.legal_entity.address", "kind": "entity_reference", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 7, "target_type_id": "address" },
                        { "predicate_id": "managed_by", "alias": "managed_by", "label_key": "fields.legal_entity.managed_by", "kind": "entity_reference", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 8, "target_type_id": "person" }
                    ]
                }
            ],
            "columns": [
                { "predicate_id": "full_name", "alias": "full_name", "label_key": "fields.legal_entity.full_name", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 },
                { "predicate_id": "has_tax_id", "alias": "has_tax_id", "label_key": "fields.legal_entity.inn", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 1 },
                { "predicate_id": "has_ogrn", "alias": "has_ogrn", "label_key": "fields.legal_entity.ogrn", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 2 }
            ],
            "default_sort": [{ "field": "full_name", "direction": "asc" }],
            "filters": [],
            "facets": [],
            "row_actions": [],
            "bulk_actions": [],
            "export_allowed": true
        }),
        "request" => serde_json::json!({
            "view_id": "request_form",
            "view_type": vtype,
            "type_id": "request",
            "schema_revision": 1,
            "name_key": "types.request",
            "sections": [
                {
                    "label_key": "sections.main",
                    "order": 0,
                    "collapsible": false,
                    "default_collapsed": false,
                    "fields": [
                        { "predicate_id": "target_entity", "alias": "target_entity", "label_key": "fields.request.target_entity", "kind": "entity_reference", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0, "target_type_id": "legal_entity" },
                        { "predicate_id": "initiator", "alias": "initiator", "label_key": "fields.request.initiator", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 1 },
                        { "predicate_id": "requested_product", "alias": "requested_product", "label_key": "fields.request.product", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 2 },
                        { "predicate_id": "requested_limit", "alias": "requested_limit", "label_key": "fields.request.limit", "kind": "money", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 3 },
                        { "predicate_id": "urgency", "alias": "urgency", "label_key": "fields.request.urgency", "kind": "enum", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 4, "options": [
                            { "value": "normal", "label_key": "requests.urgency.normal" },
                            { "value": "high", "label_key": "requests.urgency.high" },
                            { "value": "urgent", "label_key": "requests.urgency.urgent" }
                        ]},
                        { "predicate_id": "due_date", "alias": "due_date", "label_key": "fields.request.due_date", "kind": "date", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 5 },
                        { "predicate_id": "purpose", "alias": "purpose", "label_key": "fields.request.purpose", "kind": "textarea", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 6 }
                    ]
                }
            ],
            "columns": [],
            "default_sort": [],
            "filters": [],
            "facets": [],
            "row_actions": [],
            "bulk_actions": [],
            "export_allowed": false
        }),
        "address" => serde_json::json!({
            "view_id": "address_form",
            "view_type": vtype,
            "type_id": "address",
            "schema_revision": 1,
            "name_key": "types.address",
            "sections": [{
                "label_key": "sections.main",
                "order": 0,
                "collapsible": false,
                "default_collapsed": false,
                "fields": [
                    { "predicate_id": "country", "alias": "country", "label_key": "fields.address.country", "kind": "text", "required": true, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 },
                    { "predicate_id": "region", "alias": "region", "label_key": "fields.address.region", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 1 },
                    { "predicate_id": "city", "alias": "city", "label_key": "fields.address.city", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 2 },
                    { "predicate_id": "street", "alias": "street", "label_key": "fields.address.street", "kind": "text", "required": true, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 3 },
                    { "predicate_id": "postal_code", "alias": "postal_code", "label_key": "fields.address.postal_code", "kind": "text", "required": false, "read_only": false, "identity": false, "hidden": false, "min_cardinality": 0, "max_cardinality": 1, "order": 4 }
                ]
            }],
            "columns": [], "default_sort": [], "filters": [], "facets": [], "row_actions": [], "bulk_actions": [], "export_allowed": false
        }),
        _ => serde_json::json!({
            "view_id": format!("{}_form", type_id),
            "view_type": vtype,
            "type_id": type_id,
            "schema_revision": 1,
            "name_key": format!("types.{}", type_id),
            "sections": [{
                "label_key": "sections.main",
                "order": 0,
                "collapsible": false,
                "default_collapsed": false,
                "fields": [
                    { "predicate_id": "display_name", "alias": "display_name", "label_key": "fields.common.name", "kind": "text", "required": true, "read_only": false, "identity": true, "hidden": false, "min_cardinality": 1, "max_cardinality": 1, "order": 0 }
                ]
            }],
            "columns": [], "default_sort": [], "filters": [], "facets": [], "row_actions": [], "bulk_actions": [], "export_allowed": false
        }),
    };
    Json(ApiEnvelope { data: view, meta: meta() })
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
        create_check,
        list_connectors,
        create_connector,
        connector_health,
        connector_enable,
        connector_disable,
        list_schema_types,
        list_predicates,
        query_entities,
        query_entities_post,
        get_entity,
        create_entity,
        retire_entity,
        get_relations,
        create_relation,
        delete_relation,
        get_entity_checks,
        get_entity_assessment,
        get_entity_tasks,
        get_entity_documents,
        get_entity_history,
        get_entity_graph,
        list_schema_drafts,
        list_schema_published,
        create_schema_draft,
        validate_schema_draft,
        publish_schema_draft,
        impact_schema_draft,
        retire_schema_draft,
        get_type_view,
        query_requests,
        create_request,
        query_assessments,
        list_monitors,
        run_monitor,
        list_reports,
        create_report,
        dashboard_stats,
    ]
}
