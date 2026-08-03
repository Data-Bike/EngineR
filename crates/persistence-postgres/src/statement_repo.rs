//! Statement repository backed by PostgreSQL (docs/10 §6).
//!
//! Statements are append-only. Supersede/revoke update the status of the old
//! row and record the successor id (immutable history, ADR-004).

use enginer_application::ports::StatementRepository;
use enginer_domain::{
    DomainError, EntityId, Statement, StatementId, StatementStatus, TenantId,
};
use futures::future::BoxFuture;
use sqlx::postgres::PgPool;
use std::sync::Arc;

/// PostgreSQL statement repository.
#[derive(Clone)]
pub struct PgStatementRepository {
    pool: Arc<PgPool>,
}

impl PgStatementRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl StatementRepository for PgStatementRepository {
    fn insert(&self, stmt: &Statement) -> BoxFuture<'_, Result<(), DomainError>> {
        let stmt = stmt.clone();
        Box::pin(async move {
            let object_json = serde_json::to_value(&stmt.object)
                .map_err(|e| DomainError::invariant(format!("statement object encode: {e}")))?;
            let status_str = status_to_str(stmt.status);
            sqlx::query(
                "INSERT INTO core.statement
                 (tenant_id, statement_id, subject, predicate, object, object_kind,
                  valid_from, valid_to, recorded_at, provenance, confidence, status,
                  revision, fingerprint, supersedes_statement_id, revoked_by_statement_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
            )
            .bind(stmt.tenant_id.raw())
            .bind(stmt.id.raw())
            .bind(stmt.subject.raw())
            .bind(stmt.predicate.raw())
            .bind(&object_json)
            .bind(if stmt.object.is_edge() { "entity" } else { "scalar" })
            .bind(stmt.validity.0.valid_from)
            .bind(stmt.validity.0.valid_to)
            .bind(stmt.recorded.0)
            .bind(serde_json::to_value(&stmt.provenance).unwrap_or(serde_json::Value::Null))
            .bind(stmt.confidence.0.to_string())
            .bind(status_str)
            .bind(stmt.revision.0)
            .bind(stmt.fingerprint.to_hex())
            .bind(stmt.supersedes_statement_id.map(|s| s.raw()))
            .bind(stmt.revoked_by_statement_id.map(|s| s.raw()))
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("statement insert: {e}")))?;
            Ok(())
        })
    }

    fn active_for_subject(
        &self,
        tenant: TenantId,
        subject: EntityId,
        as_of: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<Vec<Statement>, DomainError>> {
        Box::pin(async move {
            let rows: Vec<(
                i64,
                uuid::Uuid,
                serde_json::Value,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
                chrono::DateTime<chrono::Utc>,
                serde_json::Value,
                String,
                String,
                i64,
                String,
                Option<i64>,
                Option<i64>,
            )> = sqlx::query_as(
                "SELECT statement_id, predicate, object, object_kind,
                        valid_from, valid_to, recorded_at, provenance, confidence,
                        status, revision, fingerprint, supersedes_statement_id,
                        revoked_by_statement_id
                 FROM core.statement
                 WHERE tenant_id = $1 AND subject = $2 AND status = 'active'
                   AND valid_from <= $3 AND (valid_to IS NULL OR valid_to >= $3)",
            )
            .bind(tenant.raw())
            .bind(subject.raw())
            .bind(as_of)
            .fetch_all(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("statement read: {e}")))?;
            Ok(rows.into_iter().map(|r| row_to_statement(tenant, subject, r)).collect())
        })
    }

    fn find_by_id(
        &self,
        tenant: TenantId,
        id: StatementId,
    ) -> BoxFuture<'_, Result<Option<Statement>, DomainError>> {
        Box::pin(async move {
            let row: Option<(
                i64,
                uuid::Uuid,
                serde_json::Value,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
                chrono::DateTime<chrono::Utc>,
                serde_json::Value,
                String,
                String,
                i64,
                String,
                Option<i64>,
                Option<i64>,
            )> = sqlx::query_as(
                "SELECT statement_id, predicate, object, object_kind,
                        valid_from, valid_to, recorded_at, provenance, confidence,
                        status, revision, fingerprint, supersedes_statement_id,
                        revoked_by_statement_id
                 FROM core.statement
                 WHERE tenant_id = $1 AND statement_id = $2",
            )
            .bind(tenant.raw())
            .bind(id.raw())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("statement find: {e}")))?;
            Ok(row.map(|r| row_to_statement(tenant, EntityId(0), r)))
        })
    }

    fn supersede(
        &self,
        tenant: TenantId,
        old: StatementId,
        by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        Box::pin(async move {
            sqlx::query(
                "UPDATE core.statement SET status = 'superseded', revoked_by_statement_id = $3
                 WHERE tenant_id = $1 AND statement_id = $2",
            )
            .bind(tenant.raw())
            .bind(old.raw())
            .bind(by.raw())
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("statement supersede: {e}")))?;
            Ok(())
        })
    }

    fn revoke(
        &self,
        tenant: TenantId,
        id: StatementId,
        by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        Box::pin(async move {
            sqlx::query(
                "UPDATE core.statement SET status = 'revoked', revoked_by_statement_id = $3
                 WHERE tenant_id = $1 AND statement_id = $2",
            )
            .bind(tenant.raw())
            .bind(id.raw())
            .bind(by.raw())
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("statement revoke: {e}")))?;
            Ok(())
        })
    }
}

fn status_to_str(s: StatementStatus) -> &'static str {
    match s {
        StatementStatus::Active => "active",
        StatementStatus::Superseded => "superseded",
        StatementStatus::Revoked => "revoked",
        StatementStatus::Quarantined => "quarantined",
    }
}

fn str_to_status(s: &str) -> StatementStatus {
    match s {
        "superseded" => StatementStatus::Superseded,
        "revoked" => StatementStatus::Revoked,
        "quarantined" => StatementStatus::Quarantined,
        _ => StatementStatus::Active,
    }
}

#[allow(clippy::type_complexity)]
fn row_to_statement(
    tenant: TenantId,
    subject: EntityId,
    r: (
        i64,
        uuid::Uuid,
        serde_json::Value,
        String,
        chrono::DateTime<chrono::Utc>,
        Option<chrono::DateTime<chrono::Utc>>,
        chrono::DateTime<chrono::Utc>,
        serde_json::Value,
        String,
        String,
        i64,
        String,
        Option<i64>,
        Option<i64>,
    ),
) -> Statement {
    let object: enginer_domain::StatementObject =
        serde_json::from_value(r.2).unwrap_or(enginer_domain::StatementObject::Scalar(
            enginer_domain::CanonicalScalar::Text {
                value_nfc: String::new(),
                language_tag: None,
            },
        ));
    let provenance: enginer_domain::ProvenanceRef =
        serde_json::from_value(r.7).unwrap_or(enginer_domain::ProvenanceRef::user(None));
    let confidence: f32 = r.8.parse().unwrap_or(1.0);
    let validity = enginer_domain::ValidPeriod {
        valid_from: r.4,
        valid_to: r.5,
    };
    let fingerprint = enginer_domain::fact_fingerprint(
        tenant,
        subject.raw(),
        enginer_domain::PredicateId(r.1),
        &object,
        &validity,
        None,
        None,
    );
    Statement {
        id: StatementId(r.0),
        tenant_id: tenant,
        subject,
        predicate: enginer_domain::PredicateId(r.1),
        object,
        validity: enginer_domain::ValidTime(validity),
        recorded: enginer_domain::RecordedTime(r.6),
        provenance,
        confidence: enginer_domain::Confidence(confidence),
        status: str_to_status(&r.9),
        revision: enginer_domain::Revision(r.10),
        fingerprint,
        supersedes_statement_id: r.12.map(StatementId),
        revoked_by_statement_id: r.13.map(StatementId),
    }
}
