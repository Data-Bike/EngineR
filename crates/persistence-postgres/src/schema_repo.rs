//! Schema repository backed by PostgreSQL (docs/10 §4).
//!
//! Reads published schema revisions from `meta.schema_revision` and the
//! associated type/predicate definitions. Writes (draft/publish) go through
//! the application layer which calls the registry + this repo.

use enginer_application::ports::SchemaRepository;
use enginer_domain::{DomainError, SchemaRevision, SchemaRevisionId, TenantId};
use futures::future::BoxFuture;
use sqlx::postgres::PgPool;
use std::sync::Arc;

/// PostgreSQL schema repository.
#[derive(Clone)]
pub struct PgSchemaRepository {
    pool: Arc<PgPool>,
}

impl PgSchemaRepository {
    /// Construct from a shared pool.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }

    /// Pool accessor (for the API app / tests).
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl SchemaRepository for PgSchemaRepository {
    fn current_published_revision(
        &self,
        tenant: TenantId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>> {
        Box::pin(async move {
            let row: Option<(i64,)> = sqlx::query_as(
                "SELECT revision_id FROM meta.schema_revision
                 WHERE tenant_id = $1 AND state = 'published'
                 ORDER BY published_at DESC NULLS LAST LIMIT 1",
            )
            .bind(tenant.raw())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("schema read: {e}")))?;
            match row {
                None => Ok(None),
                Some((rid,)) => {
                    // inline revision_by_id to avoid borrowing self in pin
                    let row: Option<(serde_json::Value,)> = sqlx::query_as(
                        "SELECT content FROM meta.schema_revision WHERE tenant_id = $1 AND revision_id = $2",
                    )
                    .bind(tenant.raw())
                    .bind(rid)
                    .fetch_optional(&*self.pool)
                    .await
                    .map_err(|e| DomainError::not_found(format!("schema read: {e}")))?;
                    match row {
                        None => Ok(None),
                        Some((v,)) => {
                            let rev: SchemaRevision = serde_json::from_value(v)
                                .map_err(|e| DomainError::invariant(format!("schema decode: {e}")))?;
                            Ok(Some(rev))
                        }
                    }
                }
            }
        })
    }

    fn revision_by_id(
        &self,
        tenant: TenantId,
        id: SchemaRevisionId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>> {
        Box::pin(async move {
            let row: Option<(serde_json::Value,)> = sqlx::query_as(
                "SELECT content FROM meta.schema_revision WHERE tenant_id = $1 AND revision_id = $2",
            )
            .bind(tenant.raw())
            .bind(id.raw())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("schema read: {e}")))?;
            match row {
                None => Ok(None),
                Some((v,)) => {
                    let rev: SchemaRevision = serde_json::from_value(v)
                        .map_err(|e| DomainError::invariant(format!("schema decode: {e}")))?;
                    Ok(Some(rev))
                }
            }
        })
    }
}
