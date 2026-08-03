-- core.statement + core.identity_key + meta.identity_policy
-- (docs/10 §6, docs/04 §6/§10).
--
-- Statements are append-only; supersede/revoke update the old row's status and
-- record the successor id. Identity keys have a unique index for dedupe
-- (docs/04 §10).

CREATE TABLE IF NOT EXISTS core.statement (
    tenant_id                uuid NOT NULL,
    statement_id             bigint NOT NULL,
    subject                  bigint NOT NULL,
    predicate                uuid NOT NULL,
    object                   jsonb NOT NULL,            -- canonical object (entity id or scalar)
    object_kind              text NOT NULL,             -- 'entity' | 'scalar'
    valid_from               timestamptz NOT NULL,
    valid_to                 timestamptz,
    recorded_at              timestamptz NOT NULL DEFAULT now(),
    provenance               jsonb NOT NULL DEFAULT '{}'::jsonb,
    confidence               text NOT NULL DEFAULT '1.0',
    status                   text NOT NULL DEFAULT 'active',  -- active|superseded|revoked|quarantined
    revision                 bigint NOT NULL DEFAULT 1,
    fingerprint              text NOT NULL,             -- BLAKE2b-256 hex (dedupe acceleration)
    supersedes_statement_id  bigint,
    revoked_by_statement_id  bigint,
    PRIMARY KEY (tenant_id, statement_id),
    FOREIGN KEY (tenant_id, subject) REFERENCES core.entity (tenant_id, entity_id)
);
CREATE INDEX IF NOT EXISTS statement_subject ON core.statement (tenant_id, subject, status, valid_from, valid_to);
CREATE INDEX IF NOT EXISTS statement_predicate ON core.statement (tenant_id, predicate, status)
    WHERE status = 'active';
-- Dedupe: same fact (subject+predicate+object+validity) is unique among active.
-- The fingerprint column accelerates lookup; full collision is verified on the
-- canonical columns (docs/04 §8).
CREATE UNIQUE INDEX IF NOT EXISTS statement_active_fact_unique
    ON core.statement (tenant_id, subject, predicate, object_kind, object, valid_from)
    WHERE status = 'active';

CREATE TABLE IF NOT EXISTS core.identity_key (
    tenant_id    uuid NOT NULL,
    type_id      uuid NOT NULL,
    policy_id    uuid NOT NULL,
    key          text NOT NULL,                         -- normalized canonical key
    entity_id    bigint NOT NULL,
    status       smallint NOT NULL DEFAULT 0,           -- 0 active, 1 retired
    created_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, type_id, policy_id, key),
    FOREIGN KEY (tenant_id, entity_id) REFERENCES core.entity (tenant_id, entity_id)
);
CREATE INDEX IF NOT EXISTS identity_key_entity ON core.identity_key (tenant_id, entity_id);

CREATE TABLE IF NOT EXISTS meta.identity_policy (
    tenant_id                   uuid NOT NULL,
    policy_id                   uuid NOT NULL DEFAULT gen_random_uuid(),
    type_id                     uuid NOT NULL,
    scope                       text NOT NULL,
    key_component_predicate_ids uuid[] NOT NULL,
    normalizer_version          text NOT NULL,
    match_mode                  text NOT NULL DEFAULT 'exact',     -- exact|fuzzy
    collision_action            text NOT NULL DEFAULT 'return_existing', -- reject|return_existing|create_candidate
    resurrection_policy         text NOT NULL DEFAULT 'reuse',     -- reuse|new_with_alias|reject
    created_at                  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, policy_id)
);
CREATE INDEX IF NOT EXISTS identity_policy_type ON meta.identity_policy (tenant_id, type_id);

-- RLS: tenant isolation.
ALTER TABLE core.statement ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.statement FORCE ROW LEVEL SECURITY;
ALTER TABLE core.identity_key ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.identity_key FORCE ROW LEVEL SECURITY;
ALTER TABLE meta.identity_policy ENABLE ROW LEVEL SECURITY;
ALTER TABLE meta.identity_policy FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_statement ON core.statement
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_identity_key ON core.identity_key
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_identity_policy ON meta.identity_policy
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE ON
    core.statement, core.identity_key, meta.identity_policy
TO enginer_app, enginer_worker;
