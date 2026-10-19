//! Demo seed: inserts the demo tenant, principals, credentials, groups,
//! memberships and a published capability policy when `ENGINER_SEED_DEMO=true`
//! (docs/18 demo profile). Idempotent — safe to run on every startup.
//!
//! Passwords come from `ENGINER_DEMO_PASSWORD` (shared) or per-user
//! `ENGINER_DEMO_PASSWORD_<LOGIN_UPPERCASE>` overrides. Default: "enginer-demo".

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHasher, SaltString};
use argon2::Argon2;
use enginer_db_runtime::{AsyncStdDbExecutor, DbError, Deadline};
use enginer_persistence::RlsContext;
use enginer_security_domain::CapabilityAction;
use sha2::Digest;
use std::sync::Arc;

/// Demo tenant.
pub const DEMO_TENANT: &str = "00000000-0000-0000-0000-000000000002";

struct Persona {
    login: &'static str,
    display: &'static str,
    principal_id: &'static str,
    groups: &'static [&'static str],
}

const PERSONAS: &[Persona] = &[
    Persona {
        login: "admin",
        display: "Администратор",
        principal_id: "00000000-0000-0000-0000-0000000000a1",
        groups: &["admins", "all_data", "superadmin"],
    },
    Persona {
        login: "security_officer",
        display: "Офицер безопасности",
        principal_id: "00000000-0000-0000-0000-0000000000a2",
        groups: &["security", "all_data"],
    },
    Persona {
        login: "manager",
        display: "Менеджер",
        principal_id: "00000000-0000-0000-0000-0000000000a3",
        groups: &["managers", "all_data"],
    },
    Persona {
        login: "risk_analyst",
        display: "Риск-аналитик",
        principal_id: "00000000-0000-0000-0000-0000000000a4",
        groups: &["analysts", "all_data"],
    },
];

/// Capability grants per capability-class group alias.
fn group_capabilities(alias: &str) -> Vec<CapabilityAction> {
    use CapabilityAction::*;
    match alias {
        "admins" => CapabilityAction::all().to_vec(),
        "managers" => vec![
            LoginUseUi,
            EntityRead,
            EntityCreate,
            EntityEdit,
            EntityRetire,
            EntityMerge,
            QueryAdvanced,
            QueryAggregate,
            CheckView,
            CheckRun,
            MonitorManage,
            ReportView,
            ReportRun,
            ReportExport,
            RiskView,
            RiskResolve,
            AssessmentView,
            BulkImport,
            BulkExport,
            AuditRead,
        ],
        "analysts" => vec![
            LoginUseUi,
            EntityRead,
            QueryAdvanced,
            QueryAggregate,
            CheckView,
            ReportView,
            RiskView,
            AssessmentView,
        ],
        "security" => vec![
            LoginUseUi,
            EntityRead,
            QueryAdvanced,
            SecurityManage,
            SecuritySimulate,
            AuditRead,
            AdminPagesAccess,
        ],
        _ => Vec::new(),
    }
}

fn hash_password(password: &str) -> Result<String, DbError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| DbError::Other {
            message: format!("argon2: {e}"),
        })
}

fn password_for(login: &str) -> String {
    let key = format!("ENGINER_DEMO_PASSWORD_{}", login.to_uppercase());
    std::env::var(&key)
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::env::var("ENGINER_DEMO_PASSWORD")
                .ok()
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "enginer-demo".to_string())
}

/// Seed the demo security dataset. Idempotent via ON CONFLICT / existence checks.
pub async fn seed_demo(exec: &Arc<AsyncStdDbExecutor>) -> Result<(), String> {
    let tenant: uuid::Uuid = DEMO_TENANT.parse().map_err(|e| format!("{e}"))?;

    // Pre-compute password hashes outside the DB job (CPU-bound).
    let mut creds: Vec<(uuid::Uuid, String, String)> = Vec::new();
    for p in PERSONAS {
        let pid: uuid::Uuid = p.principal_id.parse().map_err(|e| format!("{e}"))?;
        creds.push((
            pid,
            p.login.to_string(),
            hash_password(&password_for(p.login)).map_err(|e| e.to_string())?,
        ));
    }

    let caps_by_group: Vec<(String, Vec<String>)> = ["admins", "managers", "analysts", "security"]
        .iter()
        .map(|g| {
            (
                g.to_string(),
                group_capabilities(g)
                    .iter()
                    .map(|a| a.as_str().to_string())
                    .collect(),
            )
        })
        .collect();
    let personas: Vec<(uuid::Uuid, String, String, Vec<String>)> = PERSONAS
        .iter()
        .map(|p| {
            (
                p.principal_id.parse().unwrap(),
                p.login.to_string(),
                p.display.to_string(),
                p.groups.iter().map(|g| g.to_string()).collect(),
            )
        })
        .collect();

    let v = exec
        .run(Deadline::in_secs(30), move |pool| async move {
            let mut conn = pool.acquire().await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;

            // Tenant (core.tenant is not RLS-protected).
            sqlx::query(
                "INSERT INTO core.tenant (id, alias, default_locale, default_time_zone, data_region, status)
                 VALUES ($1, 'demo', 'ru-RU', 'Europe/Moscow', 'default', 'active')
                 ON CONFLICT (id) DO NOTHING",
            )
            .bind(tenant)
            .execute(&mut *conn)
            .await
            .map_err(|e| DbError::Other { message: e.to_string() })?;

            // Everything below is under FORCE RLS — install the tenant context
            // session-locally on this single connection.
            let rctx = RlsContext {
                tenant_id: enginer_domain::TenantId(tenant),
                principal_id: enginer_domain::PrincipalId(uuid::Uuid::nil()),
                purpose: "demo.seed".to_string(),
                policy_revision: None,
            };
            rctx.install_conn(&mut conn).await.map_err(|e| DbError::Other {
                message: e.to_string(),
            })?;
            let result = seed_inner(&mut conn, tenant, &personas, &creds, &caps_by_group).await;
            let reset = RlsContext::reset_conn(&mut conn).await;
            match (result, reset) {
                (Ok(v), Ok(())) => Ok(v),
                (Err(e), _) => Err(e),
                (Ok(_), Err(e)) => Err(DbError::Other {
                    message: e.to_string(),
                }),
            }
        })
        .await
        .map_err(|e| e.to_string())?;
    tracing::info!(seeded = %v, "demo seed complete");
    Ok(())
}

type Conn = sqlx::postgres::PgConnection;

async fn seed_inner(
    conn: &mut Conn,
    tenant: uuid::Uuid,
    personas: &[(uuid::Uuid, String, String, Vec<String>)],
    creds: &[(uuid::Uuid, String, String)],
    caps_by_group: &[(String, Vec<String>)],
) -> Result<serde_json::Value, DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };

    // Principals.
    for (pid, login, display, _groups) in personas {
        sqlx::query(
            "INSERT INTO sec.principal (principal_id, tenant_id, display_name, status, email)
             VALUES ($1, $2, $3, 'active', $4) ON CONFLICT (principal_id) DO NOTHING",
        )
        .bind(pid)
        .bind(tenant)
        .bind(display)
        .bind(format!("{login}@demo.local"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Credentials (global directory — no RLS on this table).
    for (pid, login, hash) in creds {
        sqlx::query(
            "INSERT INTO sec.principal_credential (tenant_id, principal_id, login, password_hash)
             VALUES ($1, $2, $3, $4) ON CONFLICT (login) DO NOTHING",
        )
        .bind(tenant)
        .bind(pid)
        .bind(login)
        .bind(hash)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Groups: capability groups + one data group (fixed ids for idempotency).
    let mut group_ids: std::collections::HashMap<String, uuid::Uuid> =
        std::collections::HashMap::new();
    for (alias, class, gid_s) in [
        (
            "admins",
            "capability",
            "00000000-0000-0000-0000-0000000000b1",
        ),
        (
            "managers",
            "capability",
            "00000000-0000-0000-0000-0000000000b2",
        ),
        (
            "analysts",
            "capability",
            "00000000-0000-0000-0000-0000000000b3",
        ),
        (
            "security",
            "capability",
            "00000000-0000-0000-0000-0000000000b4",
        ),
        ("all_data", "data", "00000000-0000-0000-0000-0000000000bf"),
        // Special preset roles — evaluated as flags, not via rules.
        (
            "superadmin",
            "capability",
            "00000000-0000-0000-0000-0000000000c0",
        ),
        (
            "superuser",
            "capability",
            "00000000-0000-0000-0000-0000000000c1",
        ),
    ] {
        let gid: uuid::Uuid = gid_s.parse().map_err(|e: uuid::Error| DbError::Other {
            message: e.to_string(),
        })?;
        sqlx::query(
            "INSERT INTO sec.\"group\" (group_id, tenant_id, class, alias, description)
             VALUES ($1, $2, $3, $4, $5) ON CONFLICT (group_id) DO NOTHING",
        )
        .bind(gid)
        .bind(tenant)
        .bind(class)
        .bind(alias)
        .bind(format!("demo {alias}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        group_ids.insert(alias.to_string(), gid);
    }
    // Special roles are detected by the `special` flag, not by display alias.
    sqlx::query(
        "UPDATE sec.\"group\" SET special = alias
         WHERE tenant_id = $1 AND alias IN ('superadmin','superuser')",
    )
    .bind(tenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // Memberships.
    for (pid, _login, _display, groups) in personas {
        for g in groups {
            if let Some(gid) = group_ids.get(g) {
                sqlx::query(
                    "INSERT INTO sec.principal_group_membership
                     (tenant_id, principal_id, group_id, valid_from, groups_revision)
                     SELECT $1, $2, $3, '2025-01-01 00:00:00+00', 1
                     WHERE NOT EXISTS (
                        SELECT 1 FROM sec.principal_group_membership m
                        WHERE m.tenant_id=$1 AND m.principal_id=$2 AND m.group_id=$3
                          AND m.valid_to IS NULL)",
                )
                .bind(tenant)
                .bind(pid)
                .bind(gid)
                .execute(&mut *conn)
                .await
                .map_err(db)?;
            }
        }
    }

    // Published policy revision 1.
    sqlx::query(
        "INSERT INTO sec.policy_revision (tenant_id, revision_id, state, published_at)
         VALUES ($1, 1, 'published', now()) ON CONFLICT (tenant_id, revision_id) DO NOTHING",
    )
    .bind(tenant)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // Allow rules: one row per (capability group, action).
    for (alias, actions) in caps_by_group {
        let Some(gid) = group_ids.get(alias) else {
            continue;
        };
        for action in actions {
            sqlx::query(
                "INSERT INTO sec.policy_rule
                 (tenant_id, policy_revision, effect, group_id, action, priority)
                 SELECT $1, 1, 'allow', $2, $3, 0
                 WHERE NOT EXISTS (
                    SELECT 1 FROM sec.policy_rule
                    WHERE tenant_id = $1 AND policy_revision = 1 AND group_id = $2 AND action = $3
                 )",
            )
            .bind(tenant)
            .bind(gid)
            .bind(action)
            .execute(&mut *conn)
            .await
            .map_err(db)?;
        }
    }

    // Data rule: 'data.*' allow via the data group (used by query decisions).
    if let Some(gid) = group_ids.get("all_data") {
        sqlx::query(
            "INSERT INTO sec.policy_rule
             (tenant_id, policy_revision, effect, group_id, action, priority)
             SELECT $1, 1, 'allow', $2, 'data.*', 0
             WHERE NOT EXISTS (
                SELECT 1 FROM sec.policy_rule
                WHERE tenant_id = $1 AND policy_revision = 1 AND group_id = $2 AND action = 'data.*'
             )",
        )
        .bind(tenant)
        .bind(gid)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    seed_schema(conn, tenant).await?;
    seed_relation_metadata(conn, tenant).await?;
    seed_entities(conn, tenant).await?;
    seed_operations(conn, tenant).await?;

    Ok(serde_json::json!({ "ok": true }))
}

/// Fixed uuids for operational demo rows (checks, tasks, monitors, ...).
fn ops_uuid(n: u64) -> uuid::Uuid {
    uuid::Uuid::from_u128(0x00000000_0000_7000_a000_000000000000u128 + n as u128)
}

/// Seed operational demo data: requests, checks, risk, tasks, workflows,
/// monitors, connectors and one ready report. All rows use fixed ids so the
/// seed is idempotent across restarts.
async fn seed_operations(conn: &mut Conn, tenant: uuid::Uuid) -> Result<(), DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };
    let admin: uuid::Uuid = "00000000-0000-0000-0000-0000000000a1".parse().unwrap();
    // Demo entity ids as seeded in `seed_entities`: 3 = ООО «Ромашка»,
    // 4 = АО «Вектор», 5 = Заявка entity.
    let romashka: i64 = 3;
    let vektor: i64 = 4;

    // A pending credit request for «Ромашка».
    sqlx::query(
        "INSERT INTO core.request
         (request_id, tenant_id, entity_id, initiator, status,
          requested_product, requested_limit, urgency, due_date)
         VALUES ($1,$2,$3,$4,'pending','Кредитная линия',5000000.0,'high',
                 '2026-12-01')
         ON CONFLICT (request_id) DO NOTHING",
    )
    .bind(ops_uuid(1))
    .bind(tenant)
    .bind(romashka)
    .bind(admin)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // A completed KYC check with evidence, and one pending sanction check.
    sqlx::query(
        "INSERT INTO core.check_instance
         (check_id, tenant_id, request_id, entity_id, kind, status,
          completed_at, actor, evidence, risk_level)
         VALUES ($1,$2,$3,$4,'kyc','completed',now() - interval '1 day',$5,$6,'low')
         ON CONFLICT (check_id) DO NOTHING",
    )
    .bind(ops_uuid(11))
    .bind(tenant)
    .bind(ops_uuid(1))
    .bind(romashka)
    .bind(admin)
    .bind(serde_json::json!([
        {"kind":"registry","label":"ЕГРЮЛ: запись актуальна","collected_at":"2026-10-15T09:00:00Z"},
        {"kind":"documents","label":"Уставные документы проверены","collected_at":"2026-10-15T09:05:00Z"}
    ]))
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO core.check_instance
         (check_id, tenant_id, entity_id, kind, status, actor)
         VALUES ($1,$2,$3,'sanctions','pending',$4)
         ON CONFLICT (check_id) DO NOTHING",
    )
    .bind(ops_uuid(12))
    .bind(tenant)
    .bind(vektor)
    .bind(admin)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // Risk factors + one approved assessment for «Ромашка».
    for (fid, criterion, label, level, resolved) in [
        (
            ops_uuid(21),
            "pep_match",
            "Возможное совпадение по списку ПДЛ",
            "medium",
            false,
        ),
        (
            ops_uuid(22),
            "address_mass",
            "Адрес массовой регистрации",
            "low",
            true,
        ),
    ] {
        sqlx::query(
            "INSERT INTO core.risk_factor
             (factor_id, tenant_id, entity_id, criterion_id, label, level,
              resolved, resolved_at, explanation)
             VALUES ($1,$2,$3,$4,$5,$6,$7,
                     CASE WHEN $7 THEN now() - interval '2 days' ELSE NULL END,
                     $8)
             ON CONFLICT (factor_id) DO NOTHING",
        )
        .bind(fid)
        .bind(tenant)
        .bind(romashka)
        .bind(criterion)
        .bind(label)
        .bind(level)
        .bind(resolved)
        .bind(format!("Фактор, выявленный при проверке KYC ({criterion})"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    sqlx::query(
        "INSERT INTO core.risk_assessment
         (assessment_id, tenant_id, entity_id, score, level, decision,
          status, assessor, assessed_at, rationale, approval_pending)
         VALUES ($1,$2,$3,45.0,'medium','review','approved',$4,
                 now() - interval '1 day',
                 'Умеренный риск: один незакрытый фактор среднего уровня', false)
         ON CONFLICT (assessment_id) DO NOTHING",
    )
    .bind(ops_uuid(23))
    .bind(tenant)
    .bind(romashka)
    .bind(admin)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // Tasks.
    for (tid, kind, status, title, prio) in [
        (
            ops_uuid(31),
            "review",
            "in_progress",
            "Проверить документы «Ромашка»",
            10,
        ),
        (
            ops_uuid(32),
            "contact_client",
            "pending",
            "Запросить актуальный устав",
            5,
        ),
    ] {
        sqlx::query(
            "INSERT INTO core.task
             (task_id, tenant_id, entity_id, kind, status, priority, assignee,
              due_date, title, description)
             VALUES ($1,$2,$3,$4,$5,$6,'manager',
                     now() + interval '3 days', $7, $8)
             ON CONFLICT (task_id) DO NOTHING",
        )
        .bind(tid)
        .bind(tenant)
        .bind(romashka)
        .bind(kind)
        .bind(status)
        .bind(prio)
        .bind(title)
        .bind(format!("Демо-задача: {title}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Workflow definitions (docs/13 §7) — validated by POST /workflows and
    // POST /workflows/<id>/transition.
    let wf_defs: serde_json::Value = serde_json::from_str(r##"{
        "request_processing": {"initial_state": "new", "states": [
            {"name": "new", "transitions": [{"to": "in_review", "action": "start_review"}, {"to": "cancelled", "action": "cancel"}]},
            {"name": "in_review", "transitions": [{"to": "checks_running", "action": "launch_checks"}, {"to": "cancelled", "action": "cancel"}, {"to": "new", "action": "return_to_new"}]},
            {"name": "checks_running", "transitions": [{"to": "assessment", "action": "start_assessment"}, {"to": "in_review", "action": "checks_failed_return"}]},
            {"name": "assessment", "transitions": [{"to": "decision", "action": "submit_for_decision"}, {"to": "checks_running", "action": "request_additional_checks"}]},
            {"name": "decision", "transitions": [{"to": "completed", "action": "finalize"}]},
            {"name": "completed", "transitions": []},
            {"name": "cancelled", "transitions": []}]},
        "check_execution": {"initial_state": "pending", "states": [
            {"name": "pending", "transitions": [{"to": "running", "action": "start"}, {"to": "failed", "action": "abort"}]},
            {"name": "running", "transitions": [{"to": "completed", "action": "complete"}, {"to": "failed", "action": "fail"}, {"to": "pending", "action": "retry"}]},
            {"name": "completed", "transitions": []},
            {"name": "failed", "transitions": [{"to": "pending", "action": "reschedule"}]}]},
        "risk_assessment": {"initial_state": "draft", "states": [
            {"name": "draft", "transitions": [{"to": "pending_approval", "action": "submit_for_approval"}]},
            {"name": "pending_approval", "transitions": [{"to": "approved", "action": "approve"}, {"to": "draft", "action": "reject"}]},
            {"name": "approved", "transitions": [{"to": "superseded", "action": "supersede"}]},
            {"name": "superseded", "transitions": []}]},
        "monitoring_cycle": {"initial_state": "scheduled", "states": [
            {"name": "scheduled", "transitions": [{"to": "running", "action": "start_cycle"}]},
            {"name": "running", "transitions": [{"to": "reviewed", "action": "complete_review"}, {"to": "escalated", "action": "escalate"}]},
            {"name": "reviewed", "transitions": [{"to": "scheduled", "action": "schedule_next"}, {"to": "escalated", "action": "escalate"}]},
            {"name": "escalated", "transitions": [{"to": "reviewed", "action": "resolve_escalation"}, {"to": "scheduled", "action": "schedule_next"}]}]}
    }"##).expect("workflow definitions seed is valid json");
    for (alias, def) in wf_defs.as_object().unwrap() {
        sqlx::query(
            "INSERT INTO meta.workflow_definition (tenant_id, definition_id, content)
             VALUES ($1,$2,$3)
             ON CONFLICT (tenant_id, definition_id) DO UPDATE SET content=EXCLUDED.content",
        )
        .bind(tenant)
        .bind(alias)
        .bind(def)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // A running request-processing workflow.
    sqlx::query(
        "INSERT INTO core.workflow_instance
         (workflow_id, tenant_id, definition_id, entity_id, current_state, variables)
         VALUES ($1,$2,'request_processing',$3,'in_review',
                 $4)
         ON CONFLICT (workflow_id) DO NOTHING",
    )
    .bind(ops_uuid(41))
    .bind(tenant)
    .bind(romashka)
    .bind(serde_json::json!({"request_id": ops_uuid(1).to_string(), "step": "manager_review"}))
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    // Monitoring schedules.
    for (mid, eid, name, freq) in [
        (ops_uuid(51), romashka, "ЕГРЮЛ-изменения «Ромашка»", "daily"),
        (
            ops_uuid(52),
            vektor,
            "Санкционные списки «Вектор»",
            "hourly",
        ),
    ] {
        sqlx::query(
            "INSERT INTO integration.monitor
             (monitor_id, tenant_id, name, target_entity_id, frequency, status,
              next_run_at, config)
             VALUES ($1,$2,$3,$4,$5,'active', now() + interval '1 hour',
                     $6)
             ON CONFLICT (monitor_id) DO NOTHING",
        )
        .bind(mid)
        .bind(tenant)
        .bind(name)
        .bind(eid)
        .bind(freq)
        .bind(serde_json::json!({"check_kinds": ["kyc", "sanctions"]}))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Integration connectors.
    for (cid, alias, kind, env) in [
        (ops_uuid(61), "egrul", "api", "prod"),
        (ops_uuid(62), "bki", "api", "test"),
    ] {
        sqlx::query(
            "INSERT INTO integration.connector
             (connector_id, tenant_id, alias, kind, status, config)
             VALUES ($1,$2,$3,$4,'enabled',$5)
             ON CONFLICT (tenant_id, alias) DO NOTHING",
        )
        .bind(cid)
        .bind(tenant)
        .bind(alias)
        .bind(kind)
        .bind(serde_json::json!({
            "environment": env,
            "rate_limit_rps": 10.0,
            "trust_rank": 75,
        }))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // One ready report for «Ромашка» with a real CSV payload.
    let csv = "field,value\nname,ООО «Ромашка»\ninn,7712345678\n"
        .as_bytes()
        .to_vec();
    sqlx::query(
        "INSERT INTO core.file_object
         (file_id, tenant_id, name, content_type, size, sha256, content,
          status, created_by)
         VALUES ($1,$2,'individual_check.csv','text/csv',$3,$4,$5,'ready',$6)
         ON CONFLICT (file_id) DO NOTHING",
    )
    .bind(ops_uuid(71))
    .bind(tenant)
    .bind(csv.len() as i64)
    .bind(format!(
        "sha256:{}",
        hex::encode(sha2::Sha256::digest(&csv))
    ))
    .bind(&csv)
    .bind(admin)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    sqlx::query(
        "INSERT INTO core.report_instance
         (report_id, tenant_id, kind, format, status, params, file_id,
          created_by, completed_at)
         VALUES ($1,$2,'individual_check','csv','ready',$3,$4,$5, now())
         ON CONFLICT (report_id) DO NOTHING",
    )
    .bind(ops_uuid(72))
    .bind(tenant)
    .bind(serde_json::json!({"entity_id":"00000000-0000-7000-9000-000000000003"}))
    .bind(ops_uuid(71))
    .bind(admin)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    Ok(())
}

/// Fixed uuids for the demo schema so seeds are idempotent and referable.
fn schema_uuid(n: u64) -> uuid::Uuid {
    uuid::Uuid::from_u128(0x00000000_0000_7000_8000_000000000000u128 + n as u128)
}

/// Seed the demo schema: published revision 1, types, predicates, localized
/// labels and identity policies (docs/10 §4).
async fn seed_schema(conn: &mut Conn, tenant: uuid::Uuid) -> Result<(), DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };

    // The published revision content is a full SchemaRevisionRecord so the
    // schema-admin API (drafts/published/catalog) reads it verbatim.
    let content: serde_json::Value = serde_json::from_str(r##"{
        "revision_id": "1",
        "display_name": "SKR Core 1.0",
        "description": "Базовая схема СКР: персоны, организации, заявки, проверки, документы.",
        "status": "published",
        "semantic_version": "1.0.0",
        "parent_revision_id": null,
        "version": 1,
        "etag": "\"schema:1:1\"",
        "changes": [],
        "type_count": 7,
        "predicate_count": 26,
        "created_at": "2025-01-01T00:00:00Z",
        "updated_at": "2025-01-01T00:00:00Z",
        "published_at": "2025-01-01T00:00:00Z",
        "retired_at": null,
        "content_hash": null,
        "validation": null,
        "impact": null,
        "publish_reason": "Initial seed",
        "retire_reason": null,
        "relation_groups": [
            {"id": "employment", "label": "Работа и занятость", "order": 10},
            {"id": "contacts", "label": "Контакты", "order": 20},
            {"id": "ownership", "label": "Владение и управление", "order": 30},
            {"id": "documents", "label": "Документы", "order": 40},
            {"id": "workflow", "label": "Процессы и проверки", "order": 50},
            {"id": "general", "label": "Прочие связи", "order": 90}
        ],
        "types": [
            {"type_id": "person", "stable_alias": "person", "entity_class": "data",
             "label": "Физическое лицо",
             "preview": ["full_name", "birth_date", "inn"],
             "fields": [
                {"alias": "full_name", "kind": "text", "required": true, "identity": true, "label": "ФИО"},
                {"alias": "birth_date", "kind": "date", "label": "Дата рождения"},
                {"alias": "inn", "kind": "text", "identity": true, "label": "ИНН"},
                {"alias": "email", "kind": "email", "label": "E-mail"},
                {"alias": "phone", "kind": "phone", "label": "Телефон"}]},
            {"type_id": "legal_entity", "stable_alias": "legal_entity", "entity_class": "data",
             "label": "Юридическое лицо",
             "preview": ["name", "inn", "ogrn", "address"],
             "fields": [
                {"alias": "name", "kind": "text", "required": true, "label": "Наименование"},
                {"alias": "short_name", "kind": "text", "label": "Краткое наименование"},
                {"alias": "inn", "kind": "text", "required": true, "identity": true, "label": "ИНН"},
                {"alias": "ogrn", "kind": "text", "identity": true, "label": "ОГРН"},
                {"alias": "kpp", "kind": "text", "label": "КПП"},
                {"alias": "okved", "kind": "text", "label": "ОКВЭД"},
                {"alias": "address", "kind": "text", "label": "Адрес"}]},
            {"type_id": "position", "stable_alias": "position", "entity_class": "data",
             "label": "Должность", "trans": true,
             "preview": ["title"],
             "fields": [
                {"alias": "title", "kind": "text", "required": true, "label": "Название должности"}]},
            {"type_id": "contact_point", "stable_alias": "contact_point", "entity_class": "data",
             "label": "Контактная точка",
             "preview": ["cp_kind", "cp_code", "cp_number"],
             "fields": [
                {"alias": "cp_kind", "kind": "text", "required": true, "label": "Вид контакта"},
                {"alias": "cp_code", "kind": "text", "label": "Код"},
                {"alias": "cp_number", "kind": "text", "required": true, "label": "Номер"}]},
            {"type_id": "request", "stable_alias": "request", "entity_class": "data",
             "label": "Заявка",
             "preview": ["title", "status_note"],
             "fields": [
                {"alias": "title", "kind": "text", "required": true, "label": "Заголовок"},
                {"alias": "status_note", "kind": "text", "label": "Комментарий"}]},
            {"type_id": "check", "stable_alias": "check", "entity_class": "data",
             "label": "Проверка",
             "preview": ["title"],
             "fields": [
                {"alias": "title", "kind": "text", "required": true, "label": "Заголовок"}]},
            {"type_id": "document", "stable_alias": "document", "entity_class": "data",
             "label": "Документ",
             "preview": ["title", "amount"],
             "fields": [
                {"alias": "title", "kind": "text", "required": true, "label": "Заголовок"},
                {"alias": "amount", "kind": "decimal", "label": "Сумма"}]}
        ],
        "predicates": [
            {"stable_alias": "full_name", "scalar_kind": "text", "label": "ФИО"},
            {"stable_alias": "short_name", "scalar_kind": "text", "label": "Краткое наименование"},
            {"stable_alias": "name", "scalar_kind": "text", "label": "Наименование"},
            {"stable_alias": "display_name", "scalar_kind": "text", "label": "Отображаемое имя"},
            {"stable_alias": "inn", "scalar_kind": "text", "label": "ИНН", "participates_in_identity": true},
            {"stable_alias": "ogrn", "scalar_kind": "text", "label": "ОГРН", "participates_in_identity": true},
            {"stable_alias": "kpp", "scalar_kind": "text", "label": "КПП"},
            {"stable_alias": "okved", "scalar_kind": "text", "label": "ОКВЭД"},
            {"stable_alias": "email", "scalar_kind": "email", "label": "E-mail"},
            {"stable_alias": "phone", "scalar_kind": "phone", "label": "Телефон"},
            {"stable_alias": "birth_date", "scalar_kind": "date", "label": "Дата рождения"},
            {"stable_alias": "title", "scalar_kind": "text", "label": "Заголовок"},
            {"stable_alias": "status_note", "scalar_kind": "text", "label": "Комментарий"},
            {"stable_alias": "address", "scalar_kind": "text", "label": "Адрес"},
            {"stable_alias": "amount", "scalar_kind": "decimal", "label": "Сумма"},
            {"stable_alias": "cp_kind", "scalar_kind": "text", "label": "Вид контакта"},
            {"stable_alias": "cp_code", "scalar_kind": "text", "label": "Код"},
            {"stable_alias": "cp_number", "scalar_kind": "text", "label": "Номер"},
            {"stable_alias": "works_for", "is_edge": true, "label": "Работает в",
             "domain": "person", "range": "legal_entity", "inverse_label": "Сотрудник",
             "group": "employment"},
            {"stable_alias": "owns", "is_edge": true, "label": "Владеет",
             "domain": "person", "range": "legal_entity", "inverse_label": "Владелец",
             "group": "ownership"},
            {"stable_alias": "holds_position", "is_edge": true, "label": "Занимает должность",
             "domain": "person", "range": "position", "inverse_label": "Сотрудник",
             "group": "employment"},
            {"stable_alias": "position_at", "is_edge": true, "label": "В организации",
             "domain": "position", "range": "legal_entity", "inverse_label": "Должности",
             "group": "employment"},
            {"stable_alias": "position_work_phone", "is_edge": true, "label": "Рабочий телефон",
             "domain": "position", "range": "contact_point", "inverse_label": "Должность",
             "group": "contacts"},
            {"stable_alias": "for_entity", "is_edge": true, "label": "По объекту",
             "range": "*", "inverse_label": "Имеет объект",
             "group": "workflow"},
            {"stable_alias": "performs_check", "is_edge": true, "label": "Выполняет проверку",
             "domain": "person", "range": "check", "group": "workflow"},
            {"stable_alias": "related_to", "is_edge": true, "label": "Связан с",
             "range": "*", "inverse_label": "Связан с",
             "group": "general"}
        ]
    }"##).expect("seed revision content is valid json");
    sqlx::query(
        "INSERT INTO meta.schema_revision
         (tenant_id, revision_id, public_id, state, semantic_version, content, published_at)
         VALUES ($1, 1, gen_random_uuid(), 'published', '1.0.0', $2, now())
         ON CONFLICT (tenant_id, revision_id) DO UPDATE SET content=EXCLUDED.content",
    )
    .bind(tenant)
    .bind(content)
    .execute(&mut *conn)
    .await
    .map_err(db)?;

    let types: [(u64, &str, i16, &str); 7] = [
        (1, "person", 0, "Физическое лицо"),
        (2, "legal_entity", 0, "Юридическое лицо"),
        (3, "request", 0, "Заявка"),
        (4, "check", 0, "Проверка"),
        (5, "document", 0, "Документ"),
        (6, "position", 0, "Должность"),
        (7, "contact_point", 0, "Контактная точка"),
    ];
    for (n, alias, class, label) in types {
        let tid = schema_uuid(n);
        sqlx::query(
            "INSERT INTO meta.type_definition
             (tenant_id, type_id, stable_alias, entity_class, created_revision)
             VALUES ($1,$2,$3,$4,1) ON CONFLICT (tenant_id, type_id) DO NOTHING",
        )
        .bind(tenant)
        .bind(tid)
        .bind(alias)
        .bind(class)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO meta.localized_text
             (tenant_id, resource_kind, resource_id, field, locale, text, schema_revision)
             VALUES ($1,'type',$2,'label','ru',$3,1) ON CONFLICT DO NOTHING",
        )
        .bind(tenant)
        .bind(tid)
        .bind(label)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // (seq, alias, is_edge, scalar_kind, ru label)
    let predicates: [(u64, &str, bool, Option<&str>, &str); 26] = [
        (101, "full_name", false, Some("text"), "ФИО"),
        (
            102,
            "short_name",
            false,
            Some("text"),
            "Краткое наименование",
        ),
        (103, "name", false, Some("text"), "Наименование"),
        (104, "display_name", false, Some("text"), "Отображаемое имя"),
        (105, "inn", false, Some("text"), "ИНН"),
        (106, "ogrn", false, Some("text"), "ОГРН"),
        (107, "kpp", false, Some("text"), "КПП"),
        (108, "okved", false, Some("text"), "ОКВЭД"),
        (109, "email", false, Some("email"), "E-mail"),
        (110, "phone", false, Some("phone"), "Телефон"),
        (111, "birth_date", false, Some("date"), "Дата рождения"),
        (112, "title", false, Some("text"), "Заголовок"),
        (113, "status_note", false, Some("text"), "Комментарий"),
        (114, "address", false, Some("text"), "Адрес"),
        (115, "amount", false, Some("decimal"), "Сумма"),
        (116, "cp_kind", false, Some("text"), "Вид контакта"),
        (117, "cp_code", false, Some("text"), "Код"),
        (118, "cp_number", false, Some("text"), "Номер"),
        (151, "works_for", true, None, "Работает в"),
        (152, "owns", true, None, "Владеет"),
        (153, "for_entity", true, None, "По объекту"),
        (154, "performs_check", true, None, "Выполняет проверку"),
        (155, "related_to", true, None, "Связан с"),
        (156, "holds_position", true, None, "Занимает должность"),
        (157, "position_at", true, None, "В организации"),
        (158, "position_work_phone", true, None, "Рабочий телефон"),
    ];
    for (n, alias, is_edge, kind, label) in predicates {
        let pid = schema_uuid(n);
        sqlx::query(
            "INSERT INTO meta.predicate_definition
             (tenant_id, predicate_id, stable_alias, is_edge, scalar_kind, created_revision)
             VALUES ($1,$2,$3,$4,$5,1) ON CONFLICT (tenant_id, predicate_id) DO NOTHING",
        )
        .bind(tenant)
        .bind(pid)
        .bind(alias)
        .bind(is_edge)
        .bind(kind)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO meta.localized_text
             (tenant_id, resource_kind, resource_id, field, locale, text, schema_revision)
             VALUES ($1,'predicate',$2,'label','ru',$3,1) ON CONFLICT DO NOTHING",
        )
        .bind(tenant)
        .bind(pid)
        .bind(label)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Identity policies (docs/04 §10): INN for persons and legal entities,
    // OGRN for legal entities.
    let policies: [(u64, u64, u64); 3] = [
        (201, 2, 105), // legal_entity by inn
        (202, 2, 106), // legal_entity by ogrn
        (203, 1, 105), // person by inn
    ];
    for (n, type_n, pred_n) in policies {
        sqlx::query(
            "INSERT INTO meta.identity_policy
             (tenant_id, policy_id, type_id, scope, key_component_predicate_ids,
              normalizer_version, match_mode, collision_action, resurrection_policy)
             VALUES ($1,$2,$3,'global',$4,'v1','exact','return_existing','reuse')
             ON CONFLICT DO NOTHING",
        )
        .bind(tenant)
        .bind(schema_uuid(n))
        .bind(schema_uuid(type_n))
        .bind(vec![schema_uuid(pred_n)])
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}

/// Backfill demo presentation metadata (type `preview`/`trans`, predicate
/// `group`, `relation_groups`) into a legacy published revision — one that
/// predates relation-metadata support and has no `relation_groups` key.
/// Once a revision carries `relation_groups` (seed revision, or any
/// revision published through the auditable draft flow, which inherits it),
/// the revision is considered admin-managed and is never touched again:
/// schema edits made via /admin must not be reverted by a restart.
async fn seed_relation_metadata(conn: &mut Conn, tenant: uuid::Uuid) -> Result<(), DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };
    let row: Option<(i64, serde_json::Value)> = sqlx::query_as(
        "SELECT revision_id, content FROM meta.schema_revision
         WHERE tenant_id=$1 AND state='published'",
    )
    .bind(tenant)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some((rev, mut published)) = row else {
        return Ok(());
    };
    if rev == 1 || published.get("relation_groups").is_some() {
        return Ok(());
    }
    let demo: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT content FROM meta.schema_revision WHERE tenant_id=$1 AND revision_id=1",
    )
    .bind(tenant)
    .fetch_optional(&mut *conn)
    .await
    .map_err(db)?;
    let Some(demo) = demo else { return Ok(()) };

    let alias_of = |v: &serde_json::Value| {
        v.get("stable_alias")
            .or_else(|| v.get("type_id"))
            .or_else(|| v.get("alias"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let index_by_alias = |v: &serde_json::Value, key: &str| {
        let mut m = std::collections::HashMap::new();
        for item in v
            .get(key)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let a = alias_of(item);
            if !a.is_empty() {
                m.insert(a, item.clone());
            }
        }
        m
    };
    let demo_types = index_by_alias(&demo, "types");
    let demo_preds = index_by_alias(&demo, "predicates");

    let Some(obj) = published.as_object_mut() else {
        return Ok(());
    };
    let mut changed = false;
    let type_count = {
        let types = obj.entry("types").or_insert_with(|| serde_json::json!([]));
        let mut count = 0usize;
        if let Some(arr) = types.as_array_mut() {
            for (alias, dt) in &demo_types {
                match arr.iter_mut().find(|t| alias_of(t) == *alias) {
                    Some(existing) => {
                        // Fill missing keys only — never overwrite what the
                        // published revision already declares.
                        for k in ["preview", "trans"] {
                            if existing.get(k).is_none() {
                                if let Some(v) = dt.get(k) {
                                    existing[k] = v.clone();
                                    changed = true;
                                }
                            }
                        }
                    }
                    None => {
                        arr.push(dt.clone());
                        changed = true;
                    }
                }
            }
            count = arr.len();
        }
        count
    };
    obj.insert("type_count".to_string(), serde_json::json!(type_count));
    let pred_count = {
        let preds = obj
            .entry("predicates")
            .or_insert_with(|| serde_json::json!([]));
        let mut count = 0usize;
        if let Some(arr) = preds.as_array_mut() {
            for (alias, dp) in &demo_preds {
                match arr.iter_mut().find(|p| alias_of(p) == *alias) {
                    Some(existing) => {
                        for k in ["group", "label", "inverse_label", "domain", "range"] {
                            if existing.get(k).is_none() {
                                if let Some(v) = dp.get(k) {
                                    existing[k] = v.clone();
                                    changed = true;
                                }
                            }
                        }
                    }
                    None => {
                        arr.push(dp.clone());
                        changed = true;
                    }
                }
            }
            count = arr.len();
        }
        count
    };
    obj.insert("predicate_count".to_string(), serde_json::json!(pred_count));
    if let Some(groups) = demo.get("relation_groups") {
        obj.insert("relation_groups".to_string(), groups.clone());
        changed = true;
    }
    if !changed {
        return Ok(());
    }

    sqlx::query(
        "UPDATE meta.schema_revision SET content=$1
         WHERE tenant_id=$2 AND revision_id=$3 AND state='published'",
    )
    .bind(&published)
    .bind(tenant)
    .bind(rev)
    .execute(&mut *conn)
    .await
    .map_err(db)?;
    Ok(())
}

/// Seed demo entities + statements + relations so the workspace has real data.
async fn seed_entities(conn: &mut Conn, tenant: uuid::Uuid) -> Result<(), DbError> {
    let db = |e: sqlx::Error| DbError::Other {
        message: e.to_string(),
    };
    let admin: uuid::Uuid = "00000000-0000-0000-0000-0000000000a1".parse().unwrap();

    // (entity_id, type_n, public_id)
    let entities: [(i64, u64, &str); 8] = [
        (1, 1, "00000000-0000-7000-9000-000000000001"),
        (2, 1, "00000000-0000-7000-9000-000000000002"),
        (3, 2, "00000000-0000-7000-9000-000000000003"),
        (4, 2, "00000000-0000-7000-9000-000000000004"),
        (5, 3, "00000000-0000-7000-9000-000000000005"),
        (6, 4, "00000000-0000-7000-9000-000000000006"),
        (7, 6, "00000000-0000-7000-9000-000000000007"),
        (8, 7, "00000000-0000-7000-9000-000000000008"),
    ];
    for (eid, type_n, public) in entities {
        sqlx::query(
            "INSERT INTO core.entity
             (tenant_id, entity_id, public_id, type_id, entity_class, created_by, head_revision)
             VALUES ($1,$2,$3,$4,0,$5,1) ON CONFLICT (tenant_id, entity_id) DO NOTHING",
        )
        .bind(tenant)
        .bind(eid)
        .bind(public.parse::<uuid::Uuid>().unwrap())
        .bind(schema_uuid(type_n))
        .bind(admin)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
        sqlx::query(
            "INSERT INTO core.entity_revision
             (tenant_id, entity_id, revision, schema_revision_id, actor, command)
             VALUES ($1,$2,1,1,$3,$4) ON CONFLICT DO NOTHING",
        )
        .bind(tenant)
        .bind(eid)
        .bind(admin)
        .bind(serde_json::json!({"op":"create","reason":"Демо-данные"}).to_string())
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // (statement_id, subject, predicate_seq, object jsonb)
    let text = |v: &str| serde_json::json!({"object_kind":"scalar","kind":"text","value_nfc":v,"language_tag":null});
    let stmts: Vec<(i64, i64, u64, serde_json::Value, &str)> = vec![
        (1, 1, 101, text("Иванов Иван Иванович"), "scalar"),
        (2, 1, 105, text("770708389301"), "scalar"),
        (
            3,
            1,
            111,
            serde_json::json!({"object_kind":"scalar","kind":"date","value":"1985-04-12"}),
            "scalar",
        ),
        (4, 2, 101, text("Петрова Мария Сергеевна"), "scalar"),
        (5, 2, 105, text("771234567890"), "scalar"),
        (
            6,
            2,
            109,
            serde_json::json!({"object_kind":"scalar","kind":"email","local":"m.petrova","domain":"example.ru"}),
            "scalar",
        ),
        (7, 3, 103, text("ООО «Ромашка»"), "scalar"),
        (8, 3, 105, text("7712345678"), "scalar"),
        (9, 3, 106, text("1157746123456"), "scalar"),
        (10, 3, 114, text("г. Москва, ул. Примерная, 1"), "scalar"),
        (11, 4, 103, text("АО «Вектор»"), "scalar"),
        (12, 4, 105, text("9705112233"), "scalar"),
        (13, 4, 106, text("1207700012345"), "scalar"),
        (14, 5, 112, text("Заявка на кредит №1024"), "scalar"),
        (15, 6, 112, text("KYC-проверка ООО «Ромашка»"), "scalar"),
        (16, 7, 112, text("Главный инженер"), "scalar"),
        (17, 8, 116, text("phone"), "scalar"),
        (18, 8, 117, text("+7"), "scalar"),
        (19, 8, 118, text("495 123-45-67"), "scalar"),
        (24, 8, 104, text("+7 495 123-45-67"), "scalar"),
        // edges
        (
            20,
            1,
            151,
            serde_json::json!({"object_kind":"entity","entity_id":3}),
            "entity",
        ),
        (
            21,
            2,
            151,
            serde_json::json!({"object_kind":"entity","entity_id":4}),
            "entity",
        ),
        (
            22,
            5,
            153,
            serde_json::json!({"object_kind":"entity","entity_id":3}),
            "entity",
        ),
        (
            23,
            6,
            153,
            serde_json::json!({"object_kind":"entity","entity_id":3}),
            "entity",
        ),
        // Trans-object chain: Иванов → Должность → {АО «Вектор», телефон}.
        (
            30,
            1,
            156,
            serde_json::json!({"object_kind":"entity","entity_id":7}),
            "entity",
        ),
        (
            31,
            7,
            157,
            serde_json::json!({"object_kind":"entity","entity_id":4}),
            "entity",
        ),
        (
            32,
            7,
            158,
            serde_json::json!({"object_kind":"entity","entity_id":8}),
            "entity",
        ),
    ];
    for (sid, subject, pred_n, obj, kind) in stmts {
        sqlx::query(
            "INSERT INTO core.statement
             (tenant_id, statement_id, subject, predicate, object, object_kind,
              valid_from, provenance, status, revision, fingerprint)
             SELECT $1,$2,$3,$4,$5,$6,'2026-01-01T00:00:00Z',$7,'active',1,$8
             WHERE NOT EXISTS (
                SELECT 1 FROM core.statement WHERE tenant_id=$1 AND statement_id=$2
             )",
        )
        .bind(tenant)
        .bind(sid)
        .bind(subject)
        .bind(schema_uuid(pred_n))
        .bind(&obj)
        .bind(kind)
        .bind(serde_json::json!({"kind":"user","principal_id":admin}))
        .bind(format!("seed-{sid}"))
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }

    // Identity keys for dedup (mirroring the identity policies).
    let keys: [(i64, u64, u64, &str); 4] = [
        (3, 201, 105, "7712345678"),
        (3, 202, 106, "1157746123456"),
        (4, 201, 105, "9705112233"),
        (4, 202, 106, "1207700012345"),
    ];
    for (eid, policy_n, _pred_n, key) in keys {
        sqlx::query(
            "INSERT INTO core.identity_key (tenant_id, type_id, policy_id, key, entity_id, status)
             VALUES ($1,$2,$3,$4,$5,0) ON CONFLICT DO NOTHING",
        )
        .bind(tenant)
        .bind(schema_uuid(2))
        .bind(schema_uuid(policy_n))
        .bind(key)
        .bind(eid)
        .execute(&mut *conn)
        .await
        .map_err(db)?;
    }
    Ok(())
}
