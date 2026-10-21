//! EngineR API entrypoint.

use enginer_api::{rocket_build, ApiConfig, AppState};
use std::sync::Arc;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    enginer_observability::init("info");

    let config = ApiConfig::load().unwrap_or_default();
    let db_url = std::env::var("ENGINER_DATABASE_URL")
        .unwrap_or_else(|_| config.database.url_secret_ref.clone());

    // Spawn the async-std DbExecutor bridge (ADR-007).
    let db_executor =
        enginer_db_runtime::AsyncStdDbExecutor::spawn(enginer_db_runtime::DbExecutorConfig {
            database_url: db_url,
            max_connections: config.database.max_connections,
            acquire_timeout: std::time::Duration::from_millis(config.database.acquire_timeout_ms),
            queue_capacity: config.database.db_executor_queue,
        })
        .ok()
        .map(Arc::new);

    // Demo profile: import test data (tenant, personas, groups, published
    // policy) on startup. Idempotent.
    if std::env::var("ENGINER_SEED_DEMO").as_deref() == Ok("true") {
        if let Some(exec) = &db_executor {
            match enginer_api::seed::seed_demo(exec).await {
                Ok(()) => tracing::info!("demo dataset seeded"),
                Err(e) => tracing::error!(error = %e, "demo seed failed"),
            }
        } else {
            tracing::warn!("ENGINER_SEED_DEMO set but no db executor; skipping seed");
        }
    }

    // Security client: local in-process PDP (ADR-008) backed by the real
    // PostgreSQL snapshot provider. The provider is reached through the
    // DbExecutor bridge via a scoped thread because `SnapshotProvider` is a
    // synchronous trait while the pool lives on the async-std runtime.
    let security_client: Option<Arc<dyn enginer_security_api::client::SecurityClient>> =
        db_executor.clone().map(|exec| {
            enginer_security_api::local_client(Arc::new(ExecutorSnapshotProvider { exec }))
        });

    let state = AppState::new(config, db_executor, security_client);
    enginer_api::sweep_orphaned_dump_runs();
    let _ = rocket_build(state).ignite().await?.launch().await?;
    Ok(())
}

/// Bridges the synchronous `SnapshotProvider` trait onto the `AsyncStdDbExecutor`:
/// spawns a scoped thread that `block_on`s the bridge job, which in turn runs
/// `PgSecuritySnapshotProvider` inside the async-std runtime that owns the pool.
pub struct ExecutorSnapshotProvider {
    exec: Arc<enginer_db_runtime::AsyncStdDbExecutor>,
}

impl enginer_security_api::client::SnapshotProvider for ExecutorSnapshotProvider {
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
            policy_revision: v["policy_revision"].as_i64().unwrap_or(0),
            memberships,
            rules,
        })
    }
}
