//! Security snapshot provider backed by PostgreSQL (docs/09, docs/10 §11).
//!
//! Loads the policy + membership snapshot for a tenant + principal at a given
//! revision. Implements `enginer_security_api::SnapshotProvider`.

use crate::rls::RlsContext;
use enginer_domain::{PrincipalId, TenantId};
use enginer_security_api::client::{SecurityClientError, SnapshotProvider};
use enginer_security_domain::policy::{PolicyRule, PolicyRuleEffect, ResourceSelector};
use enginer_security_domain::principal::{GroupClass, PrincipalGroupMembership};
use enginer_security_pdp::PdpSnapshot;
use sqlx::postgres::PgPool;
use std::sync::Arc;

/// PostgreSQL snapshot provider.
#[derive(Clone)]
pub struct PgSecuritySnapshotProvider {
    pool: Arc<PgPool>,
}

impl PgSecuritySnapshotProvider {
    /// Construct from a shared pool.
    pub fn new(pool: Arc<PgPool>) -> Self {
        Self { pool }
    }
}

impl SnapshotProvider for PgSecuritySnapshotProvider {
    fn snapshot(
        &self,
        tenant: TenantId,
        principal: PrincipalId,
        policy_revision: Option<i64>,
    ) -> Result<PdpSnapshot, SecurityClientError> {
        // Synchronous bridge: we run the queries on a blocking task spawned on
        // the async-std runtime via the DbExecutor in production. For Phase 0+1
        // the API app calls this from within a DbExecutor job, so a synchronous
        // async-std block_on is acceptable here.
        let pool = self.pool.clone();
        let tenant_u = tenant.raw();
        let principal_u = principal.raw();
        let result = async_std::task::block_on(async move {
            // sec.* tables are under FORCE RLS: every query must run inside a
            // transaction with the tenant context installed, otherwise the
            // snapshot silently comes back empty (fail closed — but useless).
            let mut tx = pool
                .begin()
                .await
                .map_err(|e| SecurityClientError::Unavailable {
                    message: e.to_string(),
                })?;
            RlsContext {
                tenant_id: tenant,
                principal_id: principal,
                purpose: "pdp.snapshot".to_string(),
                policy_revision,
            }
            .install_tx(&mut tx)
            .await
            .map_err(|e| SecurityClientError::Unavailable {
                message: e.to_string(),
            })?;

            // Resolve current policy revision if not pinned.
            let rev = match policy_revision {
                Some(r) => r,
                None => {
                    let row: Option<(i64,)> = sqlx::query_as(
                        "SELECT revision_id FROM sec.policy_revision
                         WHERE tenant_id = $1 AND state = 'published'
                         ORDER BY published_at DESC NULLS LAST LIMIT 1",
                    )
                    .bind(tenant_u)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(|e| SecurityClientError::Unavailable {
                        message: e.to_string(),
                    })?;
                    row.map(|(r,)| r).unwrap_or(0)
                }
            };

            // Memberships.
            let mrows: Vec<(
                uuid::Uuid,
                String,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
                i64,
            )> = sqlx::query_as(
                // Ancestor groups are effective memberships too: a role
                // inheriting from another role carries all of the parent's
                // rules (docs — роль-наследник копирует права родителя).
                // Ancestor rows inherit the direct membership's validity
                // window. UNION (set semantics) terminates parent_id cycles.
                // valid_from tolerance: writers stamp app-clock time, this
                // query uses the DB clock — allow for ±120s of clock skew.
                "WITH RECURSIVE eff AS (
                     SELECT g.group_id, g.class, g.parent_id,
                            m.valid_from, m.valid_to, m.groups_revision
                     FROM sec.principal_group_membership m
                     JOIN sec.\"group\" g ON g.group_id = m.group_id AND g.tenant_id = m.tenant_id
                     WHERE m.tenant_id = $1 AND m.principal_id = $2
                       AND (m.valid_to IS NULL OR m.valid_to >= now())
                       AND m.valid_from <= now() + interval '120 seconds'
                     UNION
                     SELECT p.group_id, p.class, p.parent_id,
                            e.valid_from, e.valid_to, e.groups_revision
                     FROM sec.\"group\" p
                     JOIN eff e ON e.parent_id = p.group_id
                     WHERE p.tenant_id = $1
                 )
                 SELECT DISTINCT group_id, class, valid_from, valid_to,
                        groups_revision FROM eff",
            )
            .bind(tenant_u)
            .bind(principal_u)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| SecurityClientError::Unavailable {
                message: e.to_string(),
            })?;
            let memberships: Vec<PrincipalGroupMembership> = mrows
                .into_iter()
                .map(|(gid, class, vf, vt, gr)| PrincipalGroupMembership {
                    principal_id: principal,
                    group_id: enginer_domain::GroupId(gid),
                    group_class: match class.as_str() {
                        "capability" => GroupClass::Capability,
                        _ => GroupClass::Data,
                    },
                    valid_from: vf,
                    valid_to: vt,
                    groups_revision: gr,
                })
                .collect();

            // Rules.
            let rrows: Vec<(
                uuid::Uuid,
                String,
                uuid::Uuid,
                String,
                i32,
                chrono::DateTime<chrono::Utc>,
                Option<chrono::DateTime<chrono::Utc>>,
            )> = sqlx::query_as(
                "SELECT policy_id, effect, group_id, action, priority, valid_from, valid_to
                 FROM sec.policy_rule
                 WHERE tenant_id = $1 AND policy_revision = $2
                   AND (valid_to IS NULL OR valid_to >= now())
                   AND valid_from <= now() + interval '120 seconds'",
            )
            .bind(tenant_u)
            .bind(rev)
            .fetch_all(&mut *tx)
            .await
            .map_err(|e| SecurityClientError::Unavailable {
                message: e.to_string(),
            })?;
            let rules: Vec<PolicyRule> = rrows
                .into_iter()
                .map(|(pid, eff, gid, action, prio, vf, vt)| PolicyRule {
                    policy_id: enginer_domain::PolicyId(pid),
                    tenant_id: tenant,
                    effect: if eff == "allow" {
                        PolicyRuleEffect::Allow
                    } else {
                        PolicyRuleEffect::Deny
                    },
                    group_id: enginer_domain::GroupId(gid),
                    action,
                    resource: ResourceSelector {
                        type_id: None,
                        entity_id: None,
                        predicate_ids: Vec::new(),
                        classification: None,
                        business_unit_scope: None,
                    },
                    row_condition: None,
                    field_mask: None,
                    priority: prio,
                    valid_from: vf,
                    valid_to: vt,
                    policy_revision: rev,
                    reason: None,
                })
                .collect();

            let snapshot = PdpSnapshot {
                policy_revision: rev,
                memberships,
                rules,
            };
            tx.commit()
                .await
                .map_err(|e| SecurityClientError::Unavailable {
                    message: e.to_string(),
                })?;
            Ok(snapshot)
        });
        result
    }
}
