//! EngineR background worker (docs/02 §1, ADR-014).
//!
//! Phase 2: poll the outbox with `FOR UPDATE SKIP LOCKED` (docs/02 §7), log
//! events, and update the projection watermark. Full job/projection
//! processing arrives in later phases.

use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    enginer_observability::init("info");
    tracing::info!("enginer-worker starting (Phase 2 skeleton)");

    let db_url = std::env::var("ENGINER_DATABASE_URL").unwrap_or_default();
    let exec = enginer_db_runtime::AsyncStdDbExecutor::spawn(
        enginer_db_runtime::DbExecutorConfig {
            database_url: db_url,
            ..Default::default()
        },
    )?;
    let exec = Arc::new(exec);

    // Outbox drain loop: every 2s, claim a batch of pending outbox rows and
    // log them. Real dispatch (HTTP/kafka) arrives in Phase 5.
    loop {
        let exec = exec.clone();
        let res = exec
            .run(
                enginer_db_runtime::Deadline::in_secs(5),
                |pool| async move {
                    // Claim a batch with FOR UPDATE SKIP LOCKED (docs/02 §7).
                    let rows: Vec<(i64, String, serde_json::Value)> = sqlx::query_as(
                        "SELECT outbox_id, event_type, payload FROM ops.outbox
                         WHERE dispatched_at IS NULL AND available_at <= now()
                         ORDER BY available_at, outbox_id
                         LIMIT 10
                         FOR UPDATE SKIP LOCKED",
                    )
                    .fetch_all(&pool)
                    .await
                    .map_err(|e| enginer_db_runtime::DbError::Other {
                        message: e.to_string(),
                    })?;

                    if rows.is_empty() {
                        return Ok(serde_json::json!({"drained": 0}));
                    }

                    let count = rows.len();
                    for (id, event_type, payload) in &rows {
                        tracing::info!(
                            outbox_id = id,
                            event_type = %event_type,
                            payload = %payload,
                            "outbox event drained"
                        );
                    }

                    // Mark the batch as dispatched.
                    let ids: Vec<i64> = rows.iter().map(|(id, _, _)| *id).collect();
                    sqlx::query("UPDATE ops.outbox SET dispatched_at = now(), attempts = attempts + 1 WHERE outbox_id = ANY($1)")
                        .bind(&ids)
                        .execute(&pool)
                        .await
                        .map_err(|e| enginer_db_runtime::DbError::Other {
                            message: e.to_string(),
                        })?;

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
