-- sec schema: principals, groups, memberships, policies, decisions, sessions
-- (docs/08 §2-4, docs/09, docs/10 §11).
--
-- Security tables are NOT tenant-isolated by RLS the same way data tables are:
-- the policy store is global to the deployment but rows carry tenant_id and
-- the application enforces tenant scoping. The PDP reads policies by
-- tenant_id + policy_revision.

CREATE TABLE IF NOT EXISTS sec.principal (
    principal_id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id           uuid NOT NULL,
    display_name        text NOT NULL,
    is_service_account  boolean NOT NULL DEFAULT false,
    status              text NOT NULL DEFAULT 'active',  -- active|disabled|locked
    created_at          timestamptz NOT NULL DEFAULT now(),
    last_seen_at        timestamptz
);
CREATE INDEX IF NOT EXISTS principal_tenant ON sec.principal (tenant_id);

CREATE TABLE IF NOT EXISTS sec."group" (
    group_id    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   uuid NOT NULL,
    class       text NOT NULL,                       -- 'capability' | 'data'
    alias       text NOT NULL,
    description text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT group_class_check CHECK (class IN ('capability','data'))
);
CREATE INDEX IF NOT EXISTS group_tenant ON sec."group" (tenant_id, class);

CREATE TABLE IF NOT EXISTS sec.principal_group_membership (
    tenant_id        uuid NOT NULL,
    principal_id     uuid NOT NULL REFERENCES sec.principal(principal_id),
    group_id         uuid NOT NULL REFERENCES sec."group"(group_id),
    valid_from       timestamptz NOT NULL DEFAULT now(),
    valid_to         timestamptz,
    groups_revision  bigint NOT NULL DEFAULT 1,
    PRIMARY KEY (tenant_id, principal_id, group_id, valid_from)
);
CREATE INDEX IF NOT EXISTS membership_principal ON sec.principal_group_membership (principal_id, valid_from, valid_to);

CREATE TABLE IF NOT EXISTS sec.policy_revision (
    tenant_id    uuid NOT NULL,
    revision_id  bigint NOT NULL,
    state        text NOT NULL DEFAULT 'draft',      -- draft|published|retired
    published_by uuid,
    published_at timestamptz,
    retired_at   timestamptz,
    PRIMARY KEY (tenant_id, revision_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS policy_revision_one_published
    ON sec.policy_revision (tenant_id) WHERE state = 'published';

CREATE TABLE IF NOT EXISTS sec.policy_rule (
    tenant_id        uuid NOT NULL,
    policy_id        uuid NOT NULL DEFAULT gen_random_uuid(),
    policy_revision  bigint NOT NULL,
    effect           text NOT NULL,                  -- 'allow' | 'deny'
    group_id         uuid NOT NULL REFERENCES sec."group"(group_id),
    action           text NOT NULL,                  -- capability string or 'data.*'
    resource         jsonb NOT NULL DEFAULT '{}'::jsonb,
    row_condition    text,
    field_mask       text,
    priority         int NOT NULL DEFAULT 0,
    valid_from       timestamptz NOT NULL DEFAULT now(),
    valid_to         timestamptz,
    reason           text,
    PRIMARY KEY (tenant_id, policy_id, policy_revision)
);
CREATE INDEX IF NOT EXISTS policy_rule_lookup ON sec.policy_rule (tenant_id, policy_revision, action, group_id);

CREATE TABLE IF NOT EXISTS sec.decision (
    decision_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    principal_id     uuid NOT NULL,
    session_id       uuid,
    auth_strength    text NOT NULL,
    action           text NOT NULL,
    resource_kind    text NOT NULL,
    resource_id      uuid,
    effect           text NOT NULL,                  -- allow|allow_with_obligations|deny|indeterminate|challenge
    obligations      jsonb NOT NULL DEFAULT '{}'::jsonb,
    policy_revision  bigint NOT NULL,
    policy_ids       uuid[] NOT NULL DEFAULT '{}',
    purpose          text,
    query_fingerprint text,
    expires_at       timestamptz NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS decision_principal ON sec.decision (tenant_id, principal_id, created_at);

CREATE TABLE IF NOT EXISTS sec.session (
    session_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id       uuid NOT NULL,
    principal_id    uuid NOT NULL,
    auth_strength   text NOT NULL,
    started_at      timestamptz NOT NULL DEFAULT now(),
    expires_at      timestamptz NOT NULL,
    ended_at        timestamptz,
    groups_revision bigint NOT NULL
);

-- RLS on sec tables: tenant isolation via app.tenant_id.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['principal','group','principal_group_membership','policy_revision','policy_rule','decision','session']
  LOOP
    EXECUTE format('ALTER TABLE sec.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE sec.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY tenant_isolation_%s ON sec.%I USING (tenant_id::text = current_setting(''app.tenant_id'', true))', t, t);
  END LOOP;
END $$;

GRANT SELECT, INSERT, UPDATE ON
    sec.principal, sec."group", sec.principal_group_membership,
    sec.policy_revision, sec.policy_rule, sec.decision, sec.session
TO enginer_app, enginer_worker;
