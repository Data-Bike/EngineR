//! Entity + identity key repositories backed by PostgreSQL (docs/10 §5-6).
//!
//! All access is parameterized; RLS context is installed by the caller (the
//! DbExecutor job) before these run. Identifiers come from the trusted catalog
//! only; user values are always bind parameters.

use enginer_application::ports::{EntityRepository, IdentityKeyRepository, IdentityPolicyRepository};
use enginer_domain::{
    DomainError, Entity, EntityClass, EntityId, EntityStatus, IdentityKey, IdentityKeyStatus,
    IdentityPolicy, PublicEntityId, TenantId, TypeId,
};
use futures::future::BoxFuture;
use sqlx::postgres::PgPool;
use std::sync::Arc;

/// PostgreSQL entity repository.
#[derive(Clone)]
pub struct PgEntityRepository {
    pool: Arc<PgPool>,
}

impl PgEntityRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl EntityRepository for PgEntityRepository {
    fn insert(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>> {
        let entity = entity.clone();
        Box::pin(async move {
            let class_i16 = entity_class_to_i16(entity.entity_class);
            sqlx::query(
                "INSERT INTO core.entity
                 (tenant_id, entity_id, public_id, entity_class, created_at, created_by,
                  retired_at, retired_by, retire_reason, head_revision, canonical_entity_id,
                  classification_id, business_unit_id)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
            )
            .bind(entity.tenant_id.raw())
            .bind(entity.entity_id.raw())
            .bind(entity.public_id.raw())
            .bind(class_i16)
            .bind(entity.created_at)
            .bind(entity.created_by)
            .bind(entity.retired_at)
            .bind(entity.retired_by)
            .bind(&entity.retire_reason)
            .bind(entity.revision.0)
            .bind(entity.canonical_entity_id.map(|e| e.raw()))
            .bind(entity.classification_id)
            .bind(entity.business_unit_id)
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("entity insert: {e}")))?;
            Ok(())
        })
    }

    fn find_by_public_id(
        &self,
        tenant: TenantId,
        public_id: PublicEntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>> {
        Box::pin(async move {
            let row: Option<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<uuid::Uuid>,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<uuid::Uuid>,
                Option<String>,
                Option<i64>,
                Option<uuid::Uuid>,
                Option<uuid::Uuid>,
            )> = sqlx::query_as(
                "SELECT entity_id, public_id, type_id, entity_class, head_revision,
                        created_at, created_by, retired_at, retired_by, retire_reason,
                        canonical_entity_id, classification_id, business_unit_id
                 FROM core.entity
                 WHERE tenant_id = $1 AND public_id = $2",
            )
            .bind(tenant.raw())
            .bind(public_id.raw())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("entity find: {e}")))?;
            Ok(row.map(|r| row_to_entity(tenant, r)))
        })
    }

    fn find_by_entity_id(
        &self,
        tenant: TenantId,
        entity_id: EntityId,
    ) -> BoxFuture<'_, Result<Option<Entity>, DomainError>> {
        Box::pin(async move {
            let row: Option<(
                i64,
                uuid::Uuid,
                uuid::Uuid,
                i16,
                i64,
                chrono::DateTime<chrono::Utc>,
                Option<uuid::Uuid>,
                Option<chrono::DateTime<chrono::Utc>>,
                Option<uuid::Uuid>,
                Option<String>,
                Option<i64>,
                Option<uuid::Uuid>,
                Option<uuid::Uuid>,
            )> = sqlx::query_as(
                "SELECT entity_id, public_id, type_id, entity_class, head_revision,
                        created_at, created_by, retired_at, retired_by, retire_reason,
                        canonical_entity_id, classification_id, business_unit_id
                 FROM core.entity
                 WHERE tenant_id = $1 AND entity_id = $2",
            )
            .bind(tenant.raw())
            .bind(entity_id.raw())
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("entity find: {e}")))?;
            Ok(row.map(|r| row_to_entity(tenant, r)))
        })
    }

    fn update(&self, entity: &Entity) -> BoxFuture<'_, Result<(), DomainError>> {
        let entity = entity.clone();
        Box::pin(async move {
            sqlx::query(
                "UPDATE core.entity
                 SET head_revision = $3, retired_at = $4, retired_by = $5,
                     retire_reason = $6, canonical_entity_id = $7
                 WHERE tenant_id = $1 AND entity_id = $2",
            )
            .bind(entity.tenant_id.raw())
            .bind(entity.entity_id.raw())
            .bind(entity.revision.0)
            .bind(entity.retired_at)
            .bind(entity.retired_by)
            .bind(&entity.retire_reason)
            .bind(entity.canonical_entity_id.map(|e| e.raw()))
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("entity update: {e}")))?;
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
            sqlx::query(
                "INSERT INTO core.entity_alias
                 (tenant_id, alias_kind, alias_value, canonical_entity_id, reason)
                 VALUES ($1, $2, $3, $4, $5)
                 ON CONFLICT DO NOTHING",
            )
            .bind(tenant.raw())
            .bind(&alias_kind)
            .bind(&alias_value)
            .bind(canonical_entity_id.raw())
            .bind(&reason)
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("alias insert: {e}")))?;
            Ok(())
        })
    }
}

fn entity_class_to_i16(c: EntityClass) -> i16 {
    match c {
        EntityClass::Data => 0,
        EntityClass::Schema => 1,
        EntityClass::Policy => 2,
        EntityClass::Workflow => 3,
        EntityClass::System => 4,
    }
}

fn i16_to_entity_class(i: i16) -> EntityClass {
    match i {
        0 => EntityClass::Data,
        1 => EntityClass::Schema,
        2 => EntityClass::Policy,
        3 => EntityClass::Workflow,
        _ => EntityClass::System,
    }
}

#[allow(clippy::type_complexity)]
fn row_to_entity(
    tenant: TenantId,
    r: (
        i64,
        uuid::Uuid,
        uuid::Uuid,
        i16,
        i64,
        chrono::DateTime<chrono::Utc>,
        Option<uuid::Uuid>,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<uuid::Uuid>,
        Option<String>,
        Option<i64>,
        Option<uuid::Uuid>,
        Option<uuid::Uuid>,
    ),
) -> Entity {
    Entity {
        tenant_id: tenant,
        entity_id: EntityId(r.0),
        public_id: PublicEntityId(r.1),
        type_id: TypeId(r.2),
        entity_class: i16_to_entity_class(r.3),
        revision: enginer_domain::Revision(r.4),
        created_at: r.5,
        created_by: r.6,
        retired_at: r.7,
        retired_by: r.8,
        retire_reason: r.9,
        canonical_entity_id: r.10.map(EntityId),
        classification_id: r.11,
        business_unit_id: r.12,
        status: if r.7.is_some() {
            EntityStatus::Retired
        } else {
            EntityStatus::Active
        },
    }
}

/// PostgreSQL identity key repository.
#[derive(Clone)]
pub struct PgIdentityKeyRepository {
    pool: Arc<PgPool>,
}

impl PgIdentityKeyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl IdentityKeyRepository for PgIdentityKeyRepository {
    fn find(
        &self,
        tenant: TenantId,
        type_id: TypeId,
        policy_id: uuid::Uuid,
        key: &str,
    ) -> BoxFuture<'_, Result<Option<IdentityKey>, DomainError>> {
        let key = key.to_string();
        Box::pin(async move {
            let row: Option<(i64, i16)> = sqlx::query_as(
                "SELECT entity_id, status FROM core.identity_key
                 WHERE tenant_id = $1 AND type_id = $2 AND policy_id = $3 AND key = $4",
            )
            .bind(tenant.raw())
            .bind(type_id.raw())
            .bind(policy_id)
            .bind(&key)
            .fetch_optional(&*self.pool)
            .await
            .map_err(|e| DomainError::not_found(format!("identity key find: {e}")))?;
            Ok(row.map(|(eid, status_i16)| IdentityKey {
                tenant_id: tenant,
                type_id,
                policy_id,
                key: key.clone(),
                entity_id: EntityId(eid),
                status: match status_i16 {
                    0 => IdentityKeyStatus::Active,
                    _ => IdentityKeyStatus::Retired,
                },
            }))
        })
    }

    fn insert(&self, key: &IdentityKey) -> BoxFuture<'_, Result<(), DomainError>> {
        let key = key.clone();
        Box::pin(async move {
            let status_i16 = match key.status {
                IdentityKeyStatus::Active => 0i16,
                IdentityKeyStatus::Retired => 1,
            };
            sqlx::query(
                "INSERT INTO core.identity_key
                 (tenant_id, type_id, policy_id, key, entity_id, status)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (tenant_id, type_id, policy_id, key) DO NOTHING",
            )
            .bind(key.tenant_id.raw())
            .bind(key.type_id.raw())
            .bind(key.policy_id)
            .bind(&key.key)
            .bind(key.entity_id.raw())
            .bind(status_i16)
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("identity key insert: {e}")))?;
            Ok(())
        })
    }

    fn set_status(
        &self,
        tenant: TenantId,
        entity_id: EntityId,
        status: IdentityKeyStatus,
    ) -> BoxFuture<'_, Result<(), DomainError>> {
        let status_i16: i16 = match status {
            IdentityKeyStatus::Active => 0,
            IdentityKeyStatus::Retired => 1,
        };
        Box::pin(async move {
            sqlx::query(
                "UPDATE core.identity_key SET status = $3
                 WHERE tenant_id = $1 AND entity_id = $2",
            )
            .bind(tenant.raw())
            .bind(entity_id.raw())
            .bind(status_i16)
            .execute(&*self.pool)
            .await
            .map_err(|e| DomainError::invariant(format!("identity key status: {e}")))?;
            Ok(())
        })
    }
}

/// PostgreSQL identity policy repository.
#[derive(Clone)]
pub struct PgIdentityPolicyRepository {
    pool: Arc<PgPool>,
}

impl PgIdentityPolicyRepository {
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl IdentityPolicyRepository for PgIdentityPolicyRepository {
    fn policies_for_type(
        &self,
        tenant: TenantId,
        type_id: TypeId,
    ) -> BoxFuture<'_, Result<Vec<IdentityPolicy>, DomainError>> {
        Box::pin(async move {
            let rows: Vec<(uuid::Uuid, String, Vec<uuid::Uuid>, String, String, String, String)> =
                sqlx::query_as(
                    "SELECT policy_id, scope, key_component_predicate_ids, normalizer_version,
                            match_mode, collision_action, resurrection_policy
                     FROM meta.identity_policy
                     WHERE tenant_id = $1 AND type_id = $2",
                )
                .bind(tenant.raw())
                .bind(type_id.raw())
                .fetch_all(&*self.pool)
                .await
                .map_err(|e| DomainError::not_found(format!("identity policy read: {e}")))?;
            Ok(rows
                .into_iter()
                .map(|r| IdentityPolicy {
                    policy_id: r.0,
                    tenant_id: tenant,
                    type_id,
                    scope: r.1,
                    key_component_predicate_ids: r.2.into_iter().map(enginer_domain::PredicateId).collect(),
                    normalizer_version: r.3,
                    match_mode: match r.4.as_str() {
                        "fuzzy" => enginer_domain::IdentityMode::Fuzzy,
                        _ => enginer_domain::IdentityMode::Exact,
                    },
                    collision_action: match r.5.as_str() {
                        "reject" => enginer_domain::CollisionAction::Reject,
                        "create_candidate" => enginer_domain::CollisionAction::CreateCandidate,
                        _ => enginer_domain::CollisionAction::ReturnExisting,
                    },
                    resurrection_policy: match r.6.as_str() {
                        "reuse" => enginer_domain::ResurrectionPolicy::Reuse,
                        "reject" => enginer_domain::ResurrectionPolicy::Reject,
                        _ => enginer_domain::ResurrectionPolicy::NewWithAlias,
                    },
                })
                .collect())
        })
    }
}
