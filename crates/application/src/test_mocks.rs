//! Mock implementations of application ports for testing use cases.
//!
//! These are in-memory, single-tenant mocks that let us test the use case
//! orchestration logic without a database. They are `#[cfg(test)]` only.

#![cfg(test)]

use crate::ports::{
    Clock, EntityRepository, IdGenerator, IdentityKeyRepository, IdentityPolicyRepository,
    Outbox, SchemaRepository, StatementRepository,
};
use enginer_domain::{
    DomainError, Entity, EntityId, IdentityKey, IdentityKeyStatus, IdentityPolicy,
    PublicEntityId, SchemaRevision, SchemaRevisionId, Statement, StatementId, TenantId, TypeId,
};
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory entity repository.
#[derive(Default)]
pub struct MockEntityRepository {
    pub by_public: Mutex<HashMap<(TenantId, uuid::Uuid), Entity>>,
    pub by_entity: Mutex<HashMap<(TenantId, i64), Entity>>,
    pub aliases: Mutex<Vec<(TenantId, String, String, EntityId, String)>>,
}

impl MockEntityRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EntityRepository for MockEntityRepository {
    fn insert(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>> {
        let entity = entity.clone();
        Box::pin(async move {
            self.by_public
                .lock()
                .unwrap()
                .insert((entity.tenant_id, entity.public_id.raw()), entity.clone());
            self.by_entity
                .lock()
                .unwrap()
                .insert((entity.tenant_id, entity.entity_id.raw()), entity);
            Ok(())
        })
    }

    fn find_by_public_id(
        &self,
        tenant: TenantId,
        public_id: PublicEntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>> {
        Box::pin(async move {
            Ok(self
                .by_public
                .lock()
                .unwrap()
                .get(&(tenant, public_id.raw()))
                .cloned())
        })
    }

    fn find_by_entity_id(
        &self,
        tenant: TenantId,
        entity_id: EntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>> {
        Box::pin(async move {
            Ok(self
                .by_entity
                .lock()
                .unwrap()
                .get(&(tenant, entity_id.raw()))
                .cloned())
        })
    }

    fn update(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>> {
        let entity = entity.clone();
        Box::pin(async move {
            self.by_public
                .lock()
                .unwrap()
                .insert((entity.tenant_id, entity.public_id.raw()), entity.clone());
            self.by_entity
                .lock()
                .unwrap()
                .insert((entity.tenant_id, entity.entity_id.raw()), entity);
            Ok(())
        })
    }

    fn add_alias(
        &self,
        tenant: TenantId,
        alias_kind: &str,
        alias_value: &str,
        canonical_entity_id: EntityId,
        reason: &str,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        let alias_kind = alias_kind.to_string();
        let alias_value = alias_value.to_string();
        let reason = reason.to_string();
        Box::pin(async move {
            self.aliases.lock().unwrap().push((
                tenant,
                alias_kind,
                alias_value,
                canonical_entity_id,
                reason,
            ));
            Ok(())
        })
    }
}

/// In-memory statement repository.
#[derive(Default)]
pub struct MockStatementRepository {
    pub statements: Mutex<Vec<Statement>>,
}

impl MockStatementRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl StatementRepository for MockStatementRepository {
    fn insert(&self, stmt: &Statement) -> BoxFuture<'_, Result<(), DomainError>> {
        let stmt = stmt.clone();
        Box::pin(async move {
            self.statements.lock().unwrap().push(stmt);
            Ok(())
        })
    }

    fn active_for_subject(
        &self,
        _tenant: TenantId,
        _subject: EntityId,
        _as_of: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<Vec<Statement>, DomainError>> {
        Box::pin(async move {
            Ok(self
                .statements
                .lock()
                .unwrap()
                .iter()
                .filter(|s| s.status == enginer_domain::StatementStatus::Active)
                .cloned()
                .collect())
        })
    }

    fn find_by_id(
        &self,
        _tenant: TenantId,
        id: StatementId,
    ) -> BoxFuture<'_, Result<Option<Statement>, DomainError>> {
        Box::pin(async move {
            Ok(self
                .statements
                .lock()
                .unwrap()
                .iter()
                .find(|s| s.id == id)
                .cloned())
        })
    }

    fn supersede(
        &self,
        _tenant: TenantId,
        _old: StatementId,
        _by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        Box::pin(async move { Ok(()) })
    }

    fn revoke(
        &self,
        _tenant: TenantId,
        _id: StatementId,
        _by: StatementId,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        Box::pin(async move { Ok(()) })
    }
}

/// In-memory identity key repository.
#[derive(Default)]
pub struct MockIdentityKeyRepository {
    pub keys: Mutex<Vec<IdentityKey>>,
}

impl MockIdentityKeyRepository {
    pub fn new() -> Self {
        Self::default()
    }
}

impl IdentityKeyRepository for MockIdentityKeyRepository {
    fn find(
        &self,
        tenant: TenantId,
        type_id: TypeId,
        policy_id: uuid::Uuid,
        key: &str,
    ) -> BoxFuture<'_, Result<Option<IdentityKey>, DomainError>> {
        let key = key.to_string();
        Box::pin(async move {
            Ok(self
                .keys
                .lock()
                .unwrap()
                .iter()
                .find(|k| {
                    k.tenant_id == tenant
                        && k.type_id == type_id
                        && k.policy_id == policy_id
                        && k.key == key
                })
                .cloned())
        })
    }

    fn insert(&self, key: &IdentityKey) -> BoxFuture<'_, Result<(), DomainError>> {
        let key = key.clone();
        Box::pin(async move {
            self.keys.lock().unwrap().push(key);
            Ok(())
        })
    }

    fn set_status(
        &self,
        _tenant: TenantId,
        entity_id: EntityId,
        status: IdentityKeyStatus,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        Box::pin(async move {
            for k in self.keys.lock().unwrap().iter_mut() {
                if k.entity_id == entity_id {
                    k.status = status;
                }
            }
            Ok(())
        })
    }
}

/// In-memory identity policy repository.
#[derive(Default)]
pub struct MockIdentityPolicyRepository {
    pub policies: Mutex<Vec<IdentityPolicy>>,
}

impl MockIdentityPolicyRepository {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_policy(&self, policy: IdentityPolicy) {
        self.policies.lock().unwrap().push(policy);
    }
}

impl IdentityPolicyRepository for MockIdentityPolicyRepository {
    fn policies_for_type(
        &self,
        tenant: TenantId,
        type_id: TypeId,
    ) -> BoxFuture<'_, Result<Vec<IdentityPolicy>, DomainError>> {
        Box::pin(async move {
            Ok(self
                .policies
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.tenant_id == tenant && p.type_id == type_id)
                .cloned()
                .collect())
        })
    }
}

/// In-memory outbox.
#[derive(Default)]
pub struct MockOutbox {
    pub events: Mutex<Vec<(TenantId, String, i64, String, serde_json::Value)>>,
}

impl MockOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn event_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    pub fn event_types(&self) -> Vec<String> {
        self.events
            .lock()
            .unwrap()
            .iter()
            .map(|(_, _, _, et, _)| et.clone())
            .collect()
    }
}

impl Outbox for MockOutbox {
    fn append(
        &self,
        tenant: TenantId,
        aggregate: String,
        revision: i64,
        event_type: String,
        payload: serde_json::Value,
    ) -> BoxFuture<'_, Result<enginer_domain::EventId, DomainError>> {
        Box::pin(async move {
            self.events
                .lock()
                .unwrap()
                .push((tenant, aggregate, revision, event_type, payload));
            Ok(enginer_domain::EventId::new())
        })
    }
}

/// Fixed-time clock for deterministic tests.
pub struct FixedClock {
    pub now: chrono::DateTime<chrono::Utc>,
}

impl FixedClock {
    pub fn new(now: chrono::DateTime<chrono::Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now_utc(&self) -> chrono::DateTime<chrono::Utc> {
        self.now
    }
}

/// Sequential id generator for deterministic tests.
pub struct SequentialIdGenerator {
    pub counter: Mutex<i64>,
}

impl SequentialIdGenerator {
    pub fn new() -> Self {
        Self {
            counter: Mutex::new(1000),
        }
    }

    fn next(&self) -> i64 {
        let mut c = self.counter.lock().unwrap();
        *c += 1;
        *c
    }
}

impl Default for SequentialIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IdGenerator for SequentialIdGenerator {
    fn new_request_id(&self) -> enginer_domain::RequestId {
        enginer_domain::RequestId::new()
    }

    fn new_entity_id(&self) -> EntityId {
        EntityId(self.next())
    }

    fn new_public_entity_id(&self) -> PublicEntityId {
        PublicEntityId(uuid::Uuid::now_v7())
    }

    fn new_statement_id(&self) -> StatementId {
        StatementId(self.next())
    }
}

/// No-op schema repository (returns None for all reads).
pub struct NoopSchemaRepository;

impl SchemaRepository for NoopSchemaRepository {
    fn current_published_revision(
        &self,
        _tenant: TenantId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>> {
        Box::pin(async move { Ok(None) })
    }

    fn revision_by_id(
        &self,
        _tenant: TenantId,
        _id: SchemaRevisionId,
    ) -> BoxFuture<'_, Result<Option<SchemaRevision>, DomainError>> {
        Box::pin(async move { Ok(None) })
    }
}

/// Helper to build an identity policy for tests.
pub fn test_identity_policy(
    tenant: TenantId,
    type_id: TypeId,
    key_predicate_ids: Vec<uuid::Uuid>,
) -> IdentityPolicy {
    IdentityPolicy {
        policy_id: uuid::Uuid::now_v7(),
        tenant_id: tenant,
        type_id,
        scope: "global".to_string(),
        key_component_predicate_ids: key_predicate_ids
            .into_iter()
            .map(enginer_domain::PredicateId)
            .collect(),
        normalizer_version: "text-v1".to_string(),
        match_mode: enginer_domain::IdentityMode::Exact,
        collision_action: enginer_domain::CollisionAction::ReturnExisting,
        resurrection_policy: enginer_domain::ResurrectionPolicy::Reuse,
    }
}

/// Helper to build a text statement object.
pub fn text_object(value: &str) -> enginer_domain::StatementObject {
    enginer_domain::StatementObject::Scalar(enginer_domain::CanonicalScalar::Text {
        value_nfc: value.to_string(),
        language_tag: None,
    })
}

/// Helper to convert an `Arc<dyn Trait>` from a concrete mock.
pub fn into_arc<T: 'static>(t: T) -> Arc<T> {
    Arc::new(t)
}
