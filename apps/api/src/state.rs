//! Application state shared across Rocket routes.

use std::sync::Arc;

use enginer_security_api::client::SecurityClient;

/// Shared application state.
pub struct AppState {
    pub config: crate::ApiConfig,
    pub db_executor: Option<Arc<enginer_db_runtime::AsyncStdDbExecutor>>,
    pub security_client: Option<Arc<dyn SecurityClient>>,
}

impl AppState {
    /// Construct with all dependencies wired.
    pub fn new(
        config: crate::ApiConfig,
        db_executor: Option<Arc<enginer_db_runtime::AsyncStdDbExecutor>>,
        security_client: Option<Arc<dyn SecurityClient>>,
    ) -> Self {
        Self {
            config,
            db_executor,
            security_client,
        }
    }
}
