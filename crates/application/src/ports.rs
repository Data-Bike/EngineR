//! Application ports (docs/05 §5). Ports accept/return domain/application
//! types only — no `PgRow`, `rocket::Request` or `serde_json::Value` crosses
//! the boundary.
//!
//! Ports return `BoxFuture` so the traits are object-safe (`dyn Trait`) — the
//! use cases hold `Arc<dyn Repository>` and the persistence layer provides
//! concrete implementations.

use enginer_domain::{
    DomainError, Entity, EntityId, IdentityKey, IdentityPolicy, PublicEntityId, SchemaRevision,
    SchemaRevisionId, Statement, StatementId, TenantId, TypeId,
};
use futures::future::BoxFuture;

/// Clock port (testable time).
pub trait Clock: Send + Sync {
    fn now_utc(&self) -> chrono::DateTime<chrono::Utc>;
}

/// Id generator port (testable ids).
pub trait IdGenerator: Send + Sync {
    fn new_request_id(&self) -> enginer_domain::RequestId;
    fn new_entity_id(&self) -> EntityId;
    fn new_public_entity_id(&self) -> PublicEntityId;
    fn new_statement_id(&self) -> StatementId;
}

/// Schema repository port (read side).
pub trait SchemaRepository: Send + Sync {
    fn current_published_revision(
        &self,
        tenant: TenantId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>>;

    fn revision_by_id(
        &self,
        tenant: TenantId,
        id: SchemaRevisionId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>>;
}

/// Entity repository port (write + read side).
pub trait EntityRepository: Send + Sync {
    /// Insert a new entity. Fails on conflict (duplicate public_id or
    /// entity_id).
    fn insert(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>>;

    /// Load an entity by its public id.
    fn find_by_public_id(
        &self,
        tenant: TenantId,
        public_id: PublicEntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>>;

    /// Load an entity by its internal id.
    fn find_by_entity_id(
        &self,
        tenant: TenantId,
        entity_id: EntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>>;

    /// Persist an updated entity (revision, status, merge pointer, etc.).
    fn update(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>>;

    /// Add an alias for a merged entity (loser public id -> canonical entity).
    fn add_alias(
        &self,
        tenant: TenantId,
        alias_kind: &str,
        alias_value: &str,
        canonical_entity_id: EntityId,
        reason: &str,
    ) -> BoxFuture<'_, Result<(), DomainError>>;
}

/// Statement repository port.
pub trait StatementRepository: Send + Sync {
    /// Insert a statement. The persistence layer computes the fingerprint and
    /// enforces the unique constraint for dedupe (docs/04 §10).
    fn insert(&self, stmt: &Statement) -> BoxFuture<'_, Result<(), DomainError>>;

    /// Load all active statements for a subject at a point in time.
    fn active_for_subject(
        &self,
        tenant: TenantId,
        subject: EntityId,
        as_of: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<Vec<Statement>, DomainError>>;

    /// Find a statement by id.
    fn find_by_id(
        &self,
        tenant: TenantId,
        id: StatementId,
    ) -> BoxFuture<'_, Result<Option<Statement>, DomainError>>;

    /// Mark a statement superseded by another (immutable history, ADR-004).
    fn supersede(
        &self,
        tenant: TenantId,
        old: StatementId,
        by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>>;

    /// Revoke a statement.
    fn revoke(
        &self,
        tenant: TenantId,
        id: StatementId,
        by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>>;
}

/// Identity key repository port (docs/04 §10).
pub trait IdentityKeyRepository: Send + Sync {
    /// Find an identity key by its normalized value. Used by CreateEntity to
    /// deduplicate concurrent creates of the same logical entity.
    fn find(
        &self,
        tenant: TenantId,
        type_id: TypeId,
        policy_id: uuid::Uuid,
        key: &str,
    ) -> BoxFuture<'_, Result<Option<IdentityKey>, DomainError>>;

    /// Insert an identity key. Fails on conflict (unique index); the caller
    /// re-reads via `find` to get the existing owner (docs/04 §10).
    fn insert(&self, key: &IdentityKey) -> BoxFuture<'_, Result<(), DomainError>>;

    /// Mark the identity key's status (active/retired) when its entity changes.
    fn set_status(
        &self,
        tenant: TenantId,
        entity_id: EntityId,
        status: enginer_domain::IdentityKeyStatus,
    ) -> BoxFuture<'_, Result<(), DomainError>>;
}

/// Identity policy repository port.
pub trait IdentityPolicyRepository: Send + Sync {
    fn policies_for_type(
        &self,
        tenant: TenantId,
        type_id: TypeId,
    ) -> BoxFuture<'_, Result<Vec<IdentityPolicy>, DomainError>>;
}

/// Audit sink port (docs/08 §11).
pub trait AuditSink: Send + Sync {
    fn record(
        &self,
        event: enginer_observability::AuditEvent,
    ) -> BoxFuture<'_, Result<(), DomainError>>;
}

/// Outbox port (docs/02 §7, ADR-014).
pub trait Outbox: Send + Sync {
    fn append(
        &self,
        tenant: TenantId,
        aggregate: String,
        revision: i64,
        event_type: String,
        payload: serde_json::Value,
    ) -> BoxFuture<'_, Result<enginer_domain::EventId, DomainError>>;
}
