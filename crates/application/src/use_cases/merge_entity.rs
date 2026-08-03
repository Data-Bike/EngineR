//! MergeEntity use case (docs/04 §11, docs/06 §6).
//!
//! Merge chooses a canonical entity, makes loser public ids aliases, points
//! `canonical_entity_id` at the winner, and writes an `EntityMerged` outbox
//! event. Repeat of the same pair is a no-op (idempotent, docs/04 §13).

use crate::error::ApplicationError;
use crate::ports::{Clock, EntityRepository, Outbox};
use enginer_domain::{PublicEntityId, TenantId};
use std::sync::Arc;

/// Command to merge `loser` into `canonical`.
#[derive(Debug, Clone)]
pub struct MergeEntityCommand {
    pub tenant_id: TenantId,
    pub canonical_public_id: PublicEntityId,
    pub loser_public_id: PublicEntityId,
    pub actor: Option<uuid::Uuid>,
    pub reason: String,
}

/// Result of merging entities.
#[derive(Debug, Clone)]
pub struct MergeEntityResult {
    pub canonical_public_id: PublicEntityId,
    pub loser_public_id: PublicEntityId,
    pub canonical_revision: i64,
    pub idempotent: bool,
}

/// The MergeEntity use case.
pub struct MergeEntity {
    pub entities: Arc<dyn EntityRepository>,
    pub clock: Arc<dyn Clock>,
    pub outbox: Arc<dyn Outbox>,
}

impl MergeEntity {
    pub async fn execute(&self, cmd: MergeEntityCommand) -> Result<MergeEntityResult, ApplicationError> {
        if cmd.canonical_public_id == cmd.loser_public_id {
            return Err(ApplicationError::Precondition {
                message: "cannot merge entity with itself".to_string(),
            });
        }

        let canonical = self
            .entities
            .find_by_public_id(cmd.tenant_id, cmd.canonical_public_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                message: "canonical entity not found".to_string(),
            })?;
        let loser = self
            .entities
            .find_by_public_id(cmd.tenant_id, cmd.loser_public_id)
            .await?
            .ok_or_else(|| ApplicationError::NotFound {
                message: "loser entity not found".to_string(),
            })?;

        // Idempotent: loser already merged into this canonical.
        if loser.canonical_entity_id == Some(canonical.entity_id) {
            return Ok(MergeEntityResult {
                canonical_public_id: cmd.canonical_public_id,
                loser_public_id: cmd.loser_public_id,
                canonical_revision: canonical.revision.0,
                idempotent: true,
            });
        }

        // Mark loser merged into canonical.
        let mut loser_upd = loser.clone();
        loser_upd.mark_merged_into(canonical.entity_id)?;
        self.entities.update(&loser_upd).await?;

        // Add alias: loser public id -> canonical entity.
        self.entities
            .add_alias(
                cmd.tenant_id,
                "public_id",
                &cmd.loser_public_id.raw().to_string(),
                canonical.entity_id,
                &cmd.reason,
            )
            .await?;

        // Outbox event.
        self.outbox
            .append(
                cmd.tenant_id,
                "entity".to_string(),
                loser_upd.revision.0,
                "entity.merged".to_string(),
                serde_json::json!({
                    "canonical_public_id": cmd.canonical_public_id.raw(),
                    "loser_public_id": cmd.loser_public_id.raw(),
                    "reason": cmd.reason,
                }),
            )
            .await?;

        Ok(MergeEntityResult {
            canonical_public_id: cmd.canonical_public_id,
            loser_public_id: cmd.loser_public_id,
            canonical_revision: canonical.revision.0,
            idempotent: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_mocks::*;
    use enginer_domain::{Entity, EntityClass, EntityId, PublicEntityId, TypeId};
    use std::sync::Arc;

    struct MergeHarness {
        uc: MergeEntity,
        entities: Arc<MockEntityRepository>,
        outbox: Arc<MockOutbox>,
    }

    fn build_use_case() -> MergeHarness {
        let entities = Arc::new(MockEntityRepository::new());
        let clock = Arc::new(FixedClock::new(chrono::Utc::now()));
        let outbox = Arc::new(MockOutbox::new());
        let uc = MergeEntity {
            entities: entities.clone(),
            clock,
            outbox: outbox.clone(),
        };
        MergeHarness { uc, entities, outbox }
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
    async fn merge_entity_happy_path() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let canonical_pid = seed_entity(&h.entities, tenant);
        let loser_pid = seed_entity(&h.entities, tenant);

        let cmd = MergeEntityCommand {
            tenant_id: tenant,
            canonical_public_id: canonical_pid,
            loser_public_id: loser_pid,
            actor: None,
            reason: "duplicate".to_string(),
        };

        let result = h.uc.execute(cmd).await.unwrap();
        assert!(!result.idempotent);
        assert_eq!(h.outbox.event_count(), 1);
        assert_eq!(h.outbox.event_types(), vec!["entity.merged"]);

        // Verify alias was added.
        assert_eq!(h.entities.aliases.lock().unwrap().len(), 1);
        let alias = h.entities.aliases.lock().unwrap()[0].clone();
        assert_eq!(alias.1, "public_id");
        assert_eq!(alias.2, loser_pid.raw().to_string());
    }

    #[tokio::test]
    async fn merge_entity_self_merge_rejected() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let pid = seed_entity(&h.entities, tenant);

        let cmd = MergeEntityCommand {
            tenant_id: tenant,
            canonical_public_id: pid,
            loser_public_id: pid,
            actor: None,
            reason: "self".to_string(),
        };

        let result = h.uc.execute(cmd).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::Precondition { .. } => {}
            other => panic!("expected Precondition, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn merge_entity_idempotent_repeat() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let canonical_pid = seed_entity(&h.entities, tenant);
        let loser_pid = seed_entity(&h.entities, tenant);

        let cmd = MergeEntityCommand {
            tenant_id: tenant,
            canonical_public_id: canonical_pid,
            loser_public_id: loser_pid,
            actor: None,
            reason: "dup".to_string(),
        };

        let r1 = h.uc.execute(cmd.clone()).await.unwrap();
        assert!(!r1.idempotent);
        let r2 = h.uc.execute(cmd).await.unwrap();
        assert!(r2.idempotent, "repeat merge must be idempotent");
        assert_eq!(h.outbox.event_count(), 1, "repeat merge must not emit second event");
    }

    #[tokio::test]
    async fn merge_entity_canonical_not_found() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let loser_pid = seed_entity(&h.entities, tenant);

        let cmd = MergeEntityCommand {
            tenant_id: tenant,
            canonical_public_id: PublicEntityId::new(),
            loser_public_id: loser_pid,
            actor: None,
            reason: "x".to_string(),
        };

        let result = h.uc.execute(cmd).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::NotFound { message } => {
                assert!(message.contains("canonical"));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn merge_entity_loser_not_found() {
        let tenant = TenantId::new();
        let h = build_use_case();
        let canonical_pid = seed_entity(&h.entities, tenant);

        let cmd = MergeEntityCommand {
            tenant_id: tenant,
            canonical_public_id: canonical_pid,
            loser_public_id: PublicEntityId::new(),
            actor: None,
            reason: "x".to_string(),
        };

        let result = h.uc.execute(cmd).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApplicationError::NotFound { message } => {
                assert!(message.contains("loser"));
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }
}
