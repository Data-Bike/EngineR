//! EngineR background worker (docs/02 §1, ADR-014).
//!
//! Polls `ops.outbox` with `FOR UPDATE SKIP LOCKED` (docs/02 §7) and
//! dispatches events: `check.execute` completes staged check instances,
//! `monitor.run` updates monitor schedules, other events are logged and
//! marked dispatched.

use std::sync::Arc;
use std::time::Duration;

use enginer_db_runtime::DbError;

fn db_err(e: sqlx::Error) -> DbError {
    DbError::Other {
        message: e.to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enginer_observability::init("info");
    tracing::info!("enginer-worker starting");

    let db_url = std::env::var("ENGINER_DATABASE_URL").unwrap_or_default();
    let exec =
        enginer_db_runtime::AsyncStdDbExecutor::spawn(enginer_db_runtime::DbExecutorConfig {
            database_url: db_url,
            ..Default::default()
        })?;
    let exec = Arc::new(exec);

    loop {
        let exec = exec.clone();
        let res = exec
            .run(
                enginer_db_runtime::Deadline::in_secs(10),
                |pool| async move {
                    let mut tx = pool.begin().await.map_err(db_err)?;

                    // Schedule due monitors (idempotent: skip monitors that
                    // already have a pending monitor.run event).
                    sqlx::query(
                        "INSERT INTO ops.outbox
                         (tenant_id, aggregate_kind, aggregate_id, revision,
                          event_type, payload)
                         SELECT m.tenant_id, 'monitor', m.monitor_id::text, 1,
                                'monitor.run',
                                jsonb_build_object('monitor_id', m.monitor_id::text)
                         FROM integration.monitor m
                         WHERE m.status = 'active'
                           AND m.next_run_at IS NOT NULL
                           AND m.next_run_at <= now()
                           AND NOT EXISTS (
                               SELECT 1 FROM ops.outbox o
                               WHERE o.dispatched_at IS NULL
                                 AND o.event_type = 'monitor.run'
                                 AND o.payload->>'monitor_id' = m.monitor_id::text
                           )",
                    )
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;

                    // Claim a batch with FOR UPDATE SKIP LOCKED (docs/02 §7).
                    let rows: Vec<(i64, uuid::Uuid, String, serde_json::Value)> = sqlx::query_as(
                        "SELECT outbox_id, tenant_id, event_type, payload FROM ops.outbox
                             WHERE dispatched_at IS NULL AND available_at <= now()
                             ORDER BY available_at, outbox_id
                             LIMIT 10
                             FOR UPDATE SKIP LOCKED",
                    )
                    .fetch_all(&mut *tx)
                    .await
                    .map_err(db_err)?;

                    if rows.is_empty() {
                        tx.commit().await.map_err(db_err)?;
                        return Ok(serde_json::json!({"drained": 0}));
                    }

                    let count = rows.len();
                    let mut ids = Vec::with_capacity(count);
                    for (id, tenant_id, event_type, payload) in &rows {
                        ids.push(*id);
                        // RLS: scope the transaction to the event's tenant.
                        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
                            .bind(tenant_id.to_string())
                            .execute(&mut *tx)
                            .await
                            .map_err(db_err)?;
                        match dispatch(&mut tx, event_type, payload).await {
                            Ok(()) => tracing::info!(
                                outbox_id = id,
                                event_type = %event_type,
                                "outbox event processed"
                            ),
                            Err(e) => tracing::warn!(
                                outbox_id = id,
                                event_type = %event_type,
                                error = %e,
                                "outbox event failed"
                            ),
                        }
                    }

                    sqlx::query(
                        "UPDATE ops.outbox SET dispatched_at = now(), attempts = attempts + 1
                         WHERE outbox_id = ANY($1)",
                    )
                    .bind(&ids)
                    .execute(&mut *tx)
                    .await
                    .map_err(db_err)?;

                    tx.commit().await.map_err(db_err)?;
                    Ok(serde_json::json!({"drained": count}))
                },
            )
            .await;

        match res {
            Ok(v) => {
                if let Some(n) = v.get("drained").and_then(|n| n.as_i64()) {
                    if n > 0 {
                        tracing::info!(drained = n, "outbox drain cycle complete");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = ?e, "outbox drain cycle failed");
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

/// Handle a single claimed outbox event inside the drain transaction.
async fn dispatch(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    event_type: &str,
    payload: &serde_json::Value,
) -> Result<(), String> {
    match event_type {
        "check.execute" => {
            let check_id = payload
                .get("check_id")
                .and_then(|v| v.as_str())
                .ok_or("payload missing check_id")?;
            let cid: uuid::Uuid = check_id.parse().map_err(|e: uuid::Error| e.to_string())?;
            let evidence = serde_json::json!([{
                "kind": "worker",
                "label": "check completed by worker",
                "collected_at": chrono::Utc::now().to_rfc3339(),
            }]);
            let changed = sqlx::query(
                "UPDATE core.check_instance
                 SET status='completed', completed_at=now(), evidence=$2
                 WHERE check_id=$1 AND status='pending'",
            )
            .bind(cid)
            .bind(&evidence)
            .execute(&mut **tx)
            .await
            .map_err(|e| e.to_string())?
            .rows_affected();
            if changed == 0 {
                return Err("check not pending or missing".into());
            }
            Ok(())
        }
        "monitor.run" => {
            let monitor_id = payload
                .get("monitor_id")
                .and_then(|v| v.as_str())
                .ok_or("payload missing monitor_id")?;
            let mid: uuid::Uuid = monitor_id.parse().map_err(|e: uuid::Error| e.to_string())?;
            let frequency: Option<String> = sqlx::query_scalar(
                "SELECT frequency FROM integration.monitor
                 WHERE monitor_id=$1 AND status='active'",
            )
            .bind(mid)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| e.to_string())?;
            let Some(frequency) = frequency else {
                return Err("monitor missing or inactive".into());
            };
            let interval = match frequency.as_str() {
                "hourly" => "1 hour",
                "weekly" => "7 days",
                "monthly" => "30 days",
                _ => "1 day",
            };
            let result = serde_json::json!({
                "run_id": uuid::Uuid::now_v7().to_string(),
                "ran_at": chrono::Utc::now().to_rfc3339(),
                "new_factors": 0,
                "checks_completed": 0,
                "checks_failed": 0,
                "escalate": false,
                "source": "scheduled",
            });
            sqlx::query(
                "UPDATE integration.monitor
                 SET last_run_at=now(), next_run_at=now()+$2::interval,
                     last_result=$3
                 WHERE monitor_id=$1",
            )
            .bind(mid)
            .bind(interval)
            .bind(&result)
            .execute(&mut **tx)
            .await
            .map_err(|e| e.to_string())?;
            Ok(())
        }
        _ => {
            tracing::info!(event_type, "no dispatch handler; marking dispatched");
            Ok(())
        }
    }
}
