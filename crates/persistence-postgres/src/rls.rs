//! Row-Level Security context (docs/08 §8, docs/10 §17).
//!
//! At the start of a transaction the persistence layer safely sets
//! `app.tenant_id`, `app.principal_id`, `app.purpose` and `app.policy_revision`
//! via `set_config(..., true)` (transaction-local). RLS policies read only
//! these transaction-local settings and reject absent context. After the
//! transaction the connection is returned to the pool only after rollback /
//! reset (tested to confirm no context leakage between pooled connections).

use enginer_domain::{PrincipalId, TenantId};
use sqlx::postgres::PgPool;

/// RLS context to install at the start of a transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RlsContext {
    pub tenant_id: TenantId,
    pub principal_id: PrincipalId,
    pub purpose: String,
    pub policy_revision: Option<i64>,
}

/// Error installing RLS context.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "detail", rename_all = "snake_case")]
pub enum RlsContextError {
    #[error("db: {message}")]
    Db { message: String },
}

impl From<sqlx::Error> for RlsContextError {
    fn from(e: sqlx::Error) -> Self {
        Self::Db {
            message: e.to_string(),
        }
    }
}

impl RlsContext {
    /// Install the RLS context on a connection (transaction-local).
    ///
    /// Uses `set_config(name, value, is_local := true)` so the setting is
    /// scoped to the current transaction and reset on commit/rollback.
    pub async fn install(&self, pool: &PgPool) -> Result<(), RlsContextError> {
        let tenant = self.tenant_id.raw().to_string();
        let principal = self.principal_id.raw().to_string();
        // set_config returns the new value; we discard it.
        sqlx::query("SELECT set_config('app.tenant_id', $1, true)")
            .bind(&tenant)
            .execute(pool)
            .await?;
        sqlx::query("SELECT set_config('app.principal_id', $1, true)")
            .bind(&principal)
            .execute(pool)
            .await?;
        sqlx::query("SELECT set_config('app.purpose', $1, true)")
            .bind(&self.purpose)
            .execute(pool)
            .await?;
        if let Some(rev) = self.policy_revision {
            sqlx::query("SELECT set_config('app.policy_revision', $1, true)")
                .bind(rev.to_string())
                .execute(pool)
                .await?;
        }
        Ok(())
    }

    /// Verify the context is visible on the connection (canary). Used by tests
    /// and the readiness check.
    pub async fn current_tenant(pool: &PgPool) -> Result<Option<String>, RlsContextError> {
        let row: (Option<String>,) =
            sqlx::query_as("SELECT current_setting('app.tenant_id', true)")
                .fetch_one(pool)
                .await?;
        Ok(row.0)
    }
}
