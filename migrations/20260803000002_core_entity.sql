-- core.tenant + entity + entity_revision + entity_alias (docs/10 §3, §5).
-- Internal bigint ids; public UUIDv7 unique. RLS enforced (docs/08 §8).

CREATE TABLE IF NOT EXISTS core.tenant (
    id              uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    alias           text NOT NULL UNIQUE,
    default_locale  text NOT NULL DEFAULT 'en-US',
    default_time_zone text NOT NULL DEFAULT 'UTC',
    data_region     text NOT NULL DEFAULT 'default',
    status          text NOT NULL DEFAULT 'active',
    created_at      timestamptz NOT NULL DEFAULT now(),
    retired_at      timestamptz
);

-- Entity (docs/10 §5).
CREATE TABLE IF NOT EXISTS core.entity (
    tenant_id           uuid NOT NULL REFERENCES core.tenant(id),
    entity_id           bigint NOT NULL,
    public_id           uuid NOT NULL,
    entity_class        smallint NOT NULL,           -- 0 data,1 schema,2 policy,3 workflow,4 system
    created_at          timestamptz NOT NULL DEFAULT now(),
    created_by          uuid,
    retired_at          timestamptz,
    retired_by          uuid,
    retire_reason       text,
    head_revision       bigint NOT NULL DEFAULT 1,
    canonical_entity_id bigint,                       -- after merge
    classification_id   uuid,
    business_unit_id    uuid,
    PRIMARY KEY (tenant_id, entity_id),
    UNIQUE (tenant_id, public_id),
    CONSTRAINT entity_no_self_canonical CHECK (canonical_entity_id IS NULL OR canonical_entity_id <> entity_id)
);
CREATE INDEX IF NOT EXISTS entity_tenant_canonical ON core.entity (tenant_id, canonical_entity_id)
    WHERE canonical_entity_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS entity_tenant_class ON core.entity (tenant_id, entity_class)
    WHERE retired_at IS NULL;

-- entity_alias: loser public ids / external aliases after merge (docs/10 §5).
CREATE TABLE IF NOT EXISTS core.entity_alias (
    tenant_id        uuid NOT NULL REFERENCES core.tenant(id),
    alias_kind       text NOT NULL,                  -- 'public_id' | 'external'
    alias_value      text NOT NULL,
    canonical_entity_id bigint NOT NULL,
    valid_from       timestamptz NOT NULL DEFAULT now(),
    valid_to        timestamptz,
    reason           text,
    UNIQUE (tenant_id, alias_kind, alias_value, valid_from)
);

-- entity_revision: append-only per-command metadata (docs/10 §5).
CREATE TABLE IF NOT EXISTS core.entity_revision (
    tenant_id          uuid NOT NULL,
    entity_id          bigint NOT NULL,
    revision           bigint NOT NULL,
    schema_revision_id bigint,
    actor              uuid,
    command            text NOT NULL,
    event_id           uuid,
    request_id         uuid,
    occurred_at        timestamptz NOT NULL DEFAULT now(),
    previous_revision  bigint,
    content_hash       bytea,
    PRIMARY KEY (tenant_id, entity_id, revision),
    FOREIGN KEY (tenant_id, entity_id) REFERENCES core.entity (tenant_id, entity_id)
);

-- RLS (docs/08 §8, docs/10 §17). FORCE ROW LEVEL SECURITY on tenant tables.
ALTER TABLE core.entity ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.entity FORCE ROW LEVEL SECURITY;
ALTER TABLE core.entity_alias ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.entity_alias FORCE ROW LEVEL SECURITY;
ALTER TABLE core.entity_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.entity_revision FORCE ROW LEVEL SECURITY;

-- Tenant isolation: only rows where app.tenant_id matches.
CREATE POLICY tenant_isolation_entity ON core.entity
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_entity_alias ON core.entity_alias
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_entity_revision ON core.entity_revision
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE ON core.tenant, core.entity, core.entity_alias, core.entity_revision TO enginer_app, enginer_worker;
