//! CreateEntity use case (docs/04 §10, docs/06 §6).
//!
//! Idempotency / dedup flow (docs/04 §10):
//! 1. Resolve the type's identity policies.
//! 2. For each policy, compute the normalized identity key from the command's
//!    statements.
//! 3. Try to insert each key; on unique-conflict, re-read the existing owner.
//! 4. If an existing active owner is found and `identity_mode == return_existing`,
//!    return `deduplicated=true` with that entity (no new statements written).
//! 5. Otherwise create the entity + statements + identity keys atomically.
//!
//! SELECT-before-INSERT without a unique constraint is forbidden (docs/04 §10).
//! The persistence layer enforces the unique index on identity keys.

use crate::error::ApplicationError;
use crate::ports::{
    Clock, EntityRepository, IdGenerator, IdentityKeyRepository, IdentityPolicyRepository,
    Outbox, SchemaRepository, StatementRepository,
};
use enginer_domain::{
    compute_identity_key, CanonicalScalar, Entity, EntityClass, EntityId, IdentityKey,
    IdentityKeyStatus, IdentityMode, PublicEntityId, Statement, StatementObject, StatementStatus,
    TenantId, TypeId,
};
use std::sync::Arc;

/// Command to create an entity with initial statements.
#[derive(Debug, Clone)]
pub struct CreateEntityCommand {
    pub tenant_id: TenantId,
    pub type_id: TypeId,
    pub schema_revision: i64,
    pub identity_mode: IdentityMode,
    pub actor: Option<uuid::Uuid>,
    /// Initial statements: (predicate_id, object).
    pub statements: Vec<(uuid::Uuid, StatementObject)>,
}

/// Dedup outcome (docs/04 §10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DedupOutcome {
    Created,
    Deduplicated,
}

/// Result of creating an entity.
#[derive(Debug, Clone)]
pub struct CreateEntityResult {
    pub entity: Entity,
    pub outcome: DedupOutcome,
    pub statements: Vec<Statement>,
}

/// The CreateEntity use case.
pub struct CreateEntity {
    pub entities: Arc<dyn EntityRepository>,
    pub statements: Arc<dyn StatementRepository>,
    pub identity_keys: Arc<dyn IdentityKeyRepository>,
    pub identity_policies: Arc<dyn IdentityPolicyRepository>,
    pub schemas: Arc<dyn SchemaRepository>,
    pub clock: Arc<dyn Clock>,
    pub ids: Arc<dyn IdGenerator>,
    pub outbox: Arc<dyn Outbox>,
}

impl CreateEntity {
    pub async fn execute(&self, cmd: CreateEntityCommand) -> Result<CreateEntityResult, ApplicationError> {
        // 1. Resolve identity policies for the type.
        let policies = self
            .identity_policies
            .policies_for_type(cmd.tenant_id, cmd.type_id)
            .await?;

        // 2. For each policy, compute the identity key from the command's
        //    statements (match predicate ids).
        let now = self.clock.now_utc();
        let mut computed_keys: Vec<(uuid::Uuid, String)> = Vec::new();
        for policy in &policies {
            let mut components: Vec<String> = Vec::with_capacity(policy.key_component_predicate_ids.len());
            for pred_id in &policy.key_component_predicate_ids {
                // Find the statement value for this predicate.
                let val = cmd.statements.iter().find_map(|(pid, obj)| {
                    if pid == &pred_id.raw() {
                        match obj {
                            StatementObject::Scalar(s) => scalar_key_component(s),
                            StatementObject::Entity(_) => None,
                        }
                    } else {
                        None
                    }
                });
                components.push(val.unwrap_or_default());
            }
            let key = compute_identity_key(&components);
            computed_keys.push((policy.policy_id, key));
        }

        // 3. Try to find an existing entity by any identity key (dedup).
        if cmd.identity_mode == IdentityMode::Exact {
            for (policy_id, key) in &computed_keys {
                if let Some(existing) = self
                    .identity_keys
                    .find(cmd.tenant_id, cmd.type_id, *policy_id, key)
                    .await?
                {
                    if existing.status == IdentityKeyStatus::Active {
                        // Deduplicate: return the existing entity.
                        let entity = self
                            .entities
                            .find_by_entity_id(cmd.tenant_id, existing.entity_id)
                            .await?
                            .ok_or_else(|| ApplicationError::Dependency {
                                message: "identity key owner missing".to_string(),
                            })?;
                        return Ok(CreateEntityResult {
                            entity,
                            outcome: DedupOutcome::Deduplicated,
                            statements: Vec::new(),
                        });
                    }
                }
            }
        }

        // 4. Create the entity.
        let entity_id = self.ids.new_entity_id();
        let public_id = self.ids.new_public_entity_id();
        let mut entity = Entity::new(
            cmd.tenant_id,
            entity_id,
            public_id,
            cmd.type_id,
            EntityClass::Data,
            cmd.actor,
        );
        entity.created_at = now;
        self.entities.insert(&entity).await?;

        // 5. Insert statements.
        let mut written: Vec<Statement> = Vec::with_capacity(cmd.statements.len());
        for (pred_uuid, obj) in &cmd.statements {
            let stmt_id = self.ids.new_statement_id();
            let stmt = Statement {
                id: stmt_id,
                tenant_id: cmd.tenant_id,
                subject: entity_id,
                predicate: enginer_domain::PredicateId(*pred_uuid),
                object: obj.clone(),
                validity: enginer_domain::ValidTime(enginer_domain::ValidPeriod::open_from(now)),
                recorded: enginer_domain::RecordedTime(now),
                provenance: enginer_domain::ProvenanceRef::user(cmd.actor),
                confidence: enginer_domain::Confidence::max(),
                status: StatementStatus::Active,
                revision: enginer_domain::Revision(1),
                fingerprint: enginer_domain::fact_fingerprint(
                    cmd.tenant_id,
                    entity_id.raw(),
                    enginer_domain::PredicateId(*pred_uuid),
                    obj,
                    &enginer_domain::ValidPeriod::open_from(now),
                    None,
                    None,
                ),
                supersedes_statement_id: None,
                revoked_by_statement_id: None,
            };
            self.statements.insert(&stmt).await?;
            written.push(stmt);
        }

        // 6. Insert identity keys.
        for (policy_id, key) in &computed_keys {
            let ik = IdentityKey {
                tenant_id: cmd.tenant_id,
                type_id: cmd.type_id,
                policy_id: *policy_id,
                key: key.clone(),
                entity_id,
                status: IdentityKeyStatus::Active,
            };
            self.identity_keys.insert(&ik).await?;
        }

        // 7. Outbox event.
        self.outbox
            .append(
                cmd.tenant_id,
                "entity".to_string(),
                entity.revision.0,
                "entity.created".to_string(),
                serde_json::json!({
                    "entity_id": entity_id.raw(),
                    "public_id": public_id.raw(),
                    "type_id": cmd.type_id.raw(),
                }),
            )
            .await?;

        Ok(CreateEntityResult {
            entity,
            outcome: DedupOutcome::Created,
            statements: written,
        })
    }
}

/// Extract a normalized key component from a canonical scalar.
fn scalar_key_component(s: &CanonicalScalar) -> Option<String> {
    match s {
        CanonicalScalar::Text { value_nfc, .. } => Some(value_nfc.clone()),
        CanonicalScalar::Email { local, domain } => Some(format!("{local}@{domain}")),
        CanonicalScalar::Phone { e164, .. } => Some(e164.clone()),
        CanonicalScalar::Integer { value } => Some(value.to_string()),
        CanonicalScalar::Uuid { value } => Some(value.to_string()),
        CanonicalScalar::Uri { value } => Some(value.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_mocks::*;
    use std::sync::Arc;

    /// Test harness: holds the use case + cloned Arcs to the mocks so we can
    /// inspect their state after the use case runs.
    struct CreateEntityHarness {
        uc: CreateEntity,
        entities: Arc<MockEntityRepository>,
        statements: Arc<MockStatementRepository>,
        identity_keys: Arc<MockIdentityKeyRepository>,
        identity_policies: Arc<MockIdentityPolicyRepository>,
        outbox: Arc<MockOutbox>,
    }

    fn build_use_case(
        policies: Vec<enginer_domain::IdentityPolicy>,
        existing_keys: Vec<IdentityKey>,
    ) -> CreateEntityHarness {
        let entities = Arc::new(MockEntityRepository::new());
        let statements = Arc::new(MockStatementRepository::new());
        let identity_keys = Arc::new(MockIdentityKeyRepository::new());
        for k in existing_keys {
            identity_keys.keys.lock().unwrap().push(k);
        }
        let identity_policies = Arc::new(MockIdentityPolicyRepository::new());
        for p in policies {
            identity_policies.policies.lock().unwrap().push(p);
        }
        let schemas = Arc::new(NoopSchemaRepository);
        let clock = Arc::new(FixedClock::new(chrono::Utc::now()));
        let ids = Arc::new(SequentialIdGenerator::new());
        let outbox = Arc::new(MockOutbox::new());
        let uc = CreateEntity {
            entities: entities.clone(),
            statements: statements.clone(),
            identity_keys: identity_keys.clone(),
            identity_policies: identity_policies.clone(),
            schemas,
            clock,
            ids,
            outbox: outbox.clone(),
        };
        CreateEntityHarness {
            uc,
            entities,
            statements,
            identity_keys,
            identity_policies,
            outbox,
        }
    }

    fn text_stmt(pred: uuid::Uuid, value: &str) -> (uuid::Uuid, StatementObject) {
        (pred, text_object(value))
    }

    #[tokio::test]
    async fn create_entity_happy_path_no_policies() {
        let h = build_use_case(vec![], vec![]);
        let tenant = TenantId::new();
        let type_id = TypeId::new();
        let pred = uuid::Uuid::now_v7();

        let cmd = CreateEntityCommand {
            tenant_id: tenant,
            type_id,
            schema_revision: 1,
            identity_mode: IdentityMode::Exact,
            actor: None,
            statements: vec![text_stmt(pred, "ООО Пример")],
        };

        let result = h.uc.execute(cmd).await.unwrap();
        assert_eq!(result.outcome, DedupOutcome::Created);
        assert_eq!(result.entity.type_id, type_id);
        assert_eq!(result.statements.len(), 1);
        assert_eq!(h.outbox.event_count(), 1);
        assert_eq!(h.outbox.event_types(), vec!["entity.created"]);
    }

    #[tokio::test]
    async fn create_entity_dedup_returns_existing() {
        let tenant = TenantId::new();
        let type_id = TypeId::new();
        let pred = uuid::Uuid::now_v7();

        // Pre-populate an existing entity + identity key.
        let existing_entity_id = EntityId::new();
        let existing_public = PublicEntityId::new();
        let existing_entity = Entity::new(
            tenant,
            existing_entity_id,
            existing_public,
            type_id,
            EntityClass::Data,
            None,
        );
        let entities = Arc::new(MockEntityRepository::new());
        entities
            .by_public
            .lock()
            .unwrap()
            .insert((tenant, existing_public.raw()), existing_entity.clone());
        entities
            .by_entity
            .lock()
            .unwrap()
            .insert((tenant, existing_entity_id.raw()), existing_entity.clone());

        let policy = test_identity_policy(tenant, type_id, vec![pred]);
        let policy_id = policy.policy_id;
        let key = compute_identity_key(&["ООО Пример".to_string()]);
        let existing_key = IdentityKey {
            tenant_id: tenant,
            type_id,
            policy_id,
            key: key.clone(),
            entity_id: existing_entity_id,
            status: IdentityKeyStatus::Active,
        };

        let identity_keys = Arc::new(MockIdentityKeyRepository::new());
        identity_keys.keys.lock().unwrap().push(existing_key);

        let identity_policies = Arc::new(MockIdentityPolicyRepository::new());
        identity_policies.policies.lock().unwrap().push(policy);

        let outbox = Arc::new(MockOutbox::new());
        let uc = CreateEntity {
            entities: entities.clone(),
            statements: Arc::new(MockStatementRepository::new()),
            identity_keys: identity_keys.clone(),
            identity_policies: identity_policies.clone(),
            schemas: Arc::new(NoopSchemaRepository),
            clock: Arc::new(FixedClock::new(chrono::Utc::now())),
            ids: Arc::new(SequentialIdGenerator::new()),
            outbox: outbox.clone(),
        };

        let cmd = CreateEntityCommand {
            tenant_id: tenant,
            type_id,
            schema_revision: 1,
            identity_mode: IdentityMode::Exact,
            actor: None,
            statements: vec![text_stmt(pred, "ООО Пример")],
        };

        let result = uc.execute(cmd).await.unwrap();
        assert_eq!(result.outcome, DedupOutcome::Deduplicated);
        assert_eq!(result.entity.entity_id, existing_entity_id);
        assert!(result.statements.is_empty(), "dedup must not write statements");
        assert_eq!(outbox.event_count(), 0, "dedup must not emit outbox event");
    }

    #[tokio::test]
    async fn create_entity_with_policy_inserts_identity_key() {
        let tenant = TenantId::new();
        let type_id = TypeId::new();
        let pred = uuid::Uuid::now_v7();

        let policy = test_identity_policy(tenant, type_id, vec![pred]);
        let h = build_use_case(vec![policy], vec![]);

        let cmd = CreateEntityCommand {
            tenant_id: tenant,
            type_id,
            schema_revision: 1,
            identity_mode: IdentityMode::Exact,
            actor: None,
            statements: vec![text_stmt(pred, "7700000000")],
        };

        let result = h.uc.execute(cmd).await.unwrap();
        assert_eq!(result.outcome, DedupOutcome::Created);
        assert_eq!(
            h.identity_keys.keys.lock().unwrap().len(),
            1,
            "identity key must be inserted"
        );
        let inserted_key = h.identity_keys.keys.lock().unwrap()[0].clone();
        assert_eq!(inserted_key.entity_id, result.entity.entity_id);
        assert_eq!(inserted_key.status, IdentityKeyStatus::Active);
    }

    #[tokio::test]
    async fn create_entity_no_statements() {
        let h = build_use_case(vec![], vec![]);
        let cmd = CreateEntityCommand {
            tenant_id: TenantId::new(),
            type_id: TypeId::new(),
            schema_revision: 1,
            identity_mode: IdentityMode::Exact,
            actor: None,
            statements: vec![],
        };

        let result = h.uc.execute(cmd).await.unwrap();
        assert_eq!(result.outcome, DedupOutcome::Created);
        assert!(result.statements.is_empty());
    }

    #[tokio::test]
    async fn create_entity_emits_outbox_with_entity_ids() {
        let h = build_use_case(vec![], vec![]);
        let type_id = TypeId::new();
        let cmd = CreateEntityCommand {
            tenant_id: TenantId::new(),
            type_id,
            schema_revision: 1,
            identity_mode: IdentityMode::Exact,
            actor: None,
            statements: vec![text_stmt(uuid::Uuid::now_v7(), "test")],
        };

        let result = h.uc.execute(cmd).await.unwrap();
        let events = h.outbox.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        let (_, aggregate, revision, event_type, payload) = &events[0];
        assert_eq!(*aggregate, "entity");
        assert_eq!(*revision, 1);
        assert_eq!(*event_type, "entity.created");
        assert_eq!(payload["type_id"], serde_json::json!(type_id.raw()));
        assert_eq!(payload["public_id"], serde_json::json!(result.entity.public_id.raw()));
    }
}
