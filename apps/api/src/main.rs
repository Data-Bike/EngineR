//! EngineR API entrypoint.

use enginer_api::{rocket_build, ApiConfig, AppState};
use rocket::config::{Config, LogLevel};
use std::sync::Arc;

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    enginer_observability::init("info");

    let config = ApiConfig::load().unwrap_or_default();
    let db_url = std::env::var("ENGINER_DATABASE_URL")
        .unwrap_or_else(|_| config.database.url_secret_ref.clone());

    // Spawn the async-std DbExecutor bridge (ADR-007).
    let db_executor = enginer_db_runtime::AsyncStdDbExecutor::spawn(
        enginer_db_runtime::DbExecutorConfig {
            database_url: db_url,
            max_connections: config.database.max_connections,
            acquire_timeout: std::time::Duration::from_millis(
                config.database.acquire_timeout_ms,
            ),
            queue_capacity: config.database.db_executor_queue,
        },
    )
    .ok()
    .map(Arc::new);

    // Security client: local in-process PDP (ADR-008). The snapshot provider
    // is wired in Phase 1 once the policy store is populated; for boot we use a
    // default-deny provider so the system fails closed.
    let security_client: Option<Arc<dyn enginer_security_api::client::SecurityClient>> =
        Some(enginer_security_api::local_client(Arc::new(
            DefaultDenySnapshotProvider,
        )));

    let state = AppState::new(config, db_executor, security_client);
    let _ = rocket_build(state).ignite().await?.launch().await?;
    Ok(())
}

/// Default-deny snapshot provider: returns an empty policy snapshot so every
/// decision is `Deny` until real policies are loaded (ADR-011, fail closed).
pub struct DefaultDenySnapshotProvider;

impl enginer_security_api::client::SnapshotProvider for DefaultDenySnapshotProvider {
    fn snapshot(
        &self,
        _tenant: enginer_domain::TenantId,
        _principal: enginer_domain::PrincipalId,
        _policy_revision: Option<i64>,
    ) -> Result<enginer_security_pdp::PdpSnapshot, enginer_security_api::client::SecurityClientError>
    {
        Ok(enginer_security_pdp::PdpSnapshot::empty(0))
    }
}
