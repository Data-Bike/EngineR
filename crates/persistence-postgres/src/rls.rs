//! Row-Level Security context (docs/08 §8, docs/10 §17).
//!
//! `app.tenant_id`, `app.principal_id`, `app.purpose` and `app.policy_revision`
//! are installed via `set_config(...)`. Two scoping modes are supported:
//!
//! - [`RlsContext::install_tx`] — transaction-local (`is_local = true`). The
//!   setting disappears automatically on commit/rollback, so nothing can leak
//!   into other pooled connections. This is the default for data access.
//! - [`RlsContext::install_conn`] — session level (`is_local = false`) on a
//!   single pooled connection for statements that cannot run inside an
//!   explicit transaction. The caller MUST run [`RlsContext::reset_conn`]
//!   before releasing the connection back to the pool.
//!
//! RLS policies read only these settings and reject absent context.

use enginer_domain::{PrincipalId, TenantId};
use sqlx::postgres::{PgConnection, PgPool};
use sqlx::Transaction;

/// RLS context to install before running tenant-scoped queries.
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
    async fn set(
        conn: &mut PgConnection,
        name: &str,
        value: String,
        is_local: bool,
    ) -> Result<(), RlsContextError> {
        // The GUC name is a compile-time constant supplied by the callers
        // below; user input only ever flows through the bind parameter.
        let stmt = format!("SELECT set_config('app.{name}', $1, {is_local})");
        sqlx::query(&stmt).bind(value).execute(&mut *conn).await?;
        Ok(())
    }

    async fn apply(&self, conn: &mut PgConnection, is_local: bool) -> Result<(), RlsContextError> {
        Self::set(
            conn,
            "tenant_id",
            self.tenant_id.raw().to_string(),
            is_local,
        )
        .await?;
        Self::set(
            conn,
            "principal_id",
            self.principal_id.raw().to_string(),
            is_local,
        )
        .await?;
        Self::set(conn, "purpose", self.purpose.clone(), is_local).await?;
        if let Some(rev) = self.policy_revision {
            Self::set(conn, "policy_revision", rev.to_string(), is_local).await?;
        }
        Ok(())
    }

    /// Install the context transaction-locally. The settings are reset by
    /// PostgreSQL on commit/rollback — nothing leaks between pooled
    /// connections.
    pub async fn install_tx(
        &self,
        tx: &mut Transaction<'_, sqlx::Postgres>,
    ) -> Result<(), RlsContextError> {
        self.apply(&mut **tx, true).await
    }

    /// Install the context at session level on a single pooled connection.
    /// The caller MUST run [`RlsContext::reset_conn`] before the connection
    /// returns to the pool, otherwise the tenant context leaks into unrelated
    /// requests.
    pub async fn install_conn(&self, conn: &mut PgConnection) -> Result<(), RlsContextError> {
        self.apply(conn, false).await
    }

    /// Clear a session-level context previously installed via
    /// [`RlsContext::install_conn`]. Setting the GUCs to `''` makes every
    /// tenant-scoped policy evaluate false (fail closed).
    pub async fn reset_conn(conn: &mut PgConnection) -> Result<(), RlsContextError> {
        for name in ["tenant_id", "principal_id", "purpose", "policy_revision"] {
            Self::set(conn, name, String::new(), false).await?;
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

/// Begin a transaction with the tenant RLS context installed. All repository
/// methods must run through this — every tenant-scoped table is under
/// FORCE RLS, so a query without `app.tenant_id` sees zero rows.
pub(crate) async fn tenant_tx(
    pool: &PgPool,
    tenant: TenantId,
) -> Result<Transaction<'static, sqlx::Postgres>, enginer_domain::DomainError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| enginer_domain::DomainError::invariant(format!("tx begin: {e}")))?;
    RlsContext {
        tenant_id: tenant,
        principal_id: PrincipalId(uuid::Uuid::nil()),
        purpose: "persistence".to_string(),
        policy_revision: None,
    }
    .install_tx(&mut tx)
    .await
    .map_err(|e| enginer_domain::DomainError::invariant(format!("rls ctx: {e}")))?;
    Ok(tx)
}
