//! RetireEntity use case (docs/06 §6, docs/04 §11).

use crate::error::ApplicationError;
use crate::ports::{Clock, EntityRepository, IdentityKeyRepository, Outbox};
use enginer_domain::{EntityStatus, IdentityKeyStatus, PublicEntityId, TenantId};
use std::sync::Arc;

/// Command to retire an entity.
#[derive(Debug, Clone)]
pub struct RetireEntityCommand {
    pub tenant_id: TenantId,
    pub public_id: PublicEntityId,
    pub actor: Option<uuid::Uuid>,
    pub reason: String,
}

/// Result of retiring an entity.
#[derive(Debug, Clone)]
pub struct RetireEntityResult {
    pub public_id: PublicEntityId,
    pub revision: i64,
    pub status: EntityStatus,
}

/// The RetireEntity use case.
pub struct RetireEntity {
    pub entities: Arc<dyn EntityRepository>,
    pub identity_keys: Arc<dyn IdentityKeyRepository>,
    pub clock: Arc<dyn Clock>,
    pub outbox: Arc<dyn Outbox>,
}

impl RetireEntity {
    pub async fn execute(&self, cmd: RetireEntityCommand) -> Result<RetireEntityResult, ApplicationError> {
        let entity = self
            .entities
            .find_by_public_id(cmd.tenant_id, cmd.public_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                message: "entity not found".to_string(),
            })?;

        let now = self.clock.now_utc();
        let prev_revision = entity.revision;
        let mut e = entity.clone();
        e.retire(cmd.actor, cmd.reason.clone(), now)?;

        // Only persist + emit if the entity actually changed (idempotent skip).
        if e.revision != prev_revision {
            self.entities.update(&e).await?;

            // Mark identity keys retired so future creates can resurrect or reject.
            self.identity_keys
                .set_status(cmd.tenant_id, e.entity_id, IdentityKeyStatus::Retired)
                .await?;

            self.outbox
                .append(
                    cmd.tenant_id,
                    "entity".to_string(),
                    e.revision.0,
                    "entity.retired".to_string(),
                    serde_json::json!({
                        "public_id": cmd.public_id.raw(),
                        "reason": cmd.reason,
                    }),
                )
                .await?;
        }

        Ok(RetireEntityResult {
            public_id: cmd.public_id,
            revision: e.revision.0,
            status: e.status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_mocks::*;
    use enginer_domain::{Entity, EntityClass, EntityId, EntityStatus, PublicEntityId, TypeId};
    use std::sync::Arc;

    struct RetireHarness {
        uc: RetireEntity,
        entities: Arc<MockEntityRepository>,
        outbox: Arc<MockOutbox>,
    }

    fn build_use_case() -> RetireHarness {
        let entities = Arc::new(MockEntityRepository::new());
        let identity_keys = Arc::new(MockIdentityKeyRepository::new());
        let clock = Arc::new(FixedClock::new(chrono::Utc::now()));
        let outbox = Arc::new(MockOutbox::new());
        let uc = RetireEntity {
            entities: entities.clone(),
            identity_keys,
            clock,
            outbox: outbox.clone(),
        };
        RetireHarness { uc, entities, outbox }
    }

    fn seed_entity(entities: &MockEntityRepository, tenant: TenantId) -> PublicEntityId {
        let entity_id = EntityId::new();
        let public_id = PublicEntityId::new();
        let entity = Entity::new(
            tenant,
            entity_id,
            public_id,
            TypeId::new(),
            EntityClass::Data,
            None,
        );
        entities
            .by_public
            .lock()
            .unwrap()
            .insert((tenant, public_id.raw()), entity.clone());
        entities
            .by_entity
            .lock()
            .unwrap()
            .insert((tenant, entity_id.raw()), entity);
        public_id
    }

    #[tokio::test]
    async fn retire_entity_happy_path() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let public_id = seed_entity(&h.entities, tenant);

        let cmd = RetireEntityCommand {
            tenant_id: tenant,
            public_id,
            actor: None,
            reason: "duplicate".to_string(),
        };

        let result = h.uc.execute(cmd).await.unwrap();
        assert_eq!(result.status, EntityStatus::Retired);
        assert!(result.revision > 1, "revision must bump on retire");
        assert_eq!(h.outbox.event_count(), 1);
        assert_eq!(h.outbox.event_types(), vec!["entity.retired"]);

        // Verify entity is persisted as retired.
        let stored = h
            .entities
            .by_public
            .lock()
            .unwrap()
            .get(&(tenant, public_id.raw()))
            .unwrap()
            .clone();
        assert!(stored.is_retired());
    }

    #[tokio::test]
    async fn retire_entity_not_found() {
        let h = build_use_case();
        let cmd = RetireEntityCommand {
            tenant_id: TenantId::new(),
            public_id: PublicEntityId::new(),
            actor: None,
            reason: "x".to_string(),
        };
        let result = h.uc.execute(cmd).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::NotFound { .. } => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn retire_entity_idempotent_no_double_bump() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let public_id = seed_entity(&h.entities, tenant);

        let cmd1 = RetireEntityCommand {
            tenant_id: tenant,
            public_id,
            actor: None,
            reason: "dup".to_string(),
        };
        let r1 = h.uc.execute(cmd1).await.unwrap();
        let cmd2 = RetireEntityCommand {
            tenant_id: tenant,
            public_id,
            actor: None,
            reason: "dup".to_string(),
        };
        let r2 = h.uc.execute(cmd2).await.unwrap();

        // Second retire is idempotent: same revision, no extra outbox event.
        assert_eq!(r1.revision, r2.revision);
        assert_eq!(h.outbox.event_count(), 1);
    }
}
