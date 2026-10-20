-- Catalogs: second-level data isolation inside a tenant (docs/28).
--
-- core.catalog is a first-class row (not a graph entity). Every entity,
-- statement, type/predicate definition and schema revision belongs to
-- exactly one catalog. Alias/type/predicate namespaces become
-- per-catalog; the one-published-revision uniqueness becomes
-- per-(tenant, catalog).
--
-- Backfill: `general`/`sys` are created for every tenant; `wh` ("Склад")
-- only when wh_* types exist. wh_* types/predicates (and the revisions/
-- workflows defining them) move to `wh`; entities adopt the catalog of
-- their own type; statements adopt the catalog of their subject entity.
-- Everything else stays in `general`.
--
-- NOTE: backfill runs inside a per-tenant loop with `app.tenant_id` set —
-- the touched tables already have FORCE ROW LEVEL SECURITY, and the
-- migrate role is not BYPASSRLS, so unscoped updates would silently
-- affect zero rows.

CREATE TABLE IF NOT EXISTS core.catalog (
    tenant_id   uuid NOT NULL REFERENCES core.tenant(id),
    catalog_id  bigint NOT NULL,
    public_id   uuid NOT NULL,
    alias       text NOT NULL,
    name        text NOT NULL,
    description text NOT NULL DEFAULT '',
    status      text NOT NULL DEFAULT 'active',   -- active | retired
    settings    jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now(),
    created_by  uuid,
    retired_at  timestamptz,
    PRIMARY KEY (tenant_id, catalog_id),
    UNIQUE (tenant_id, public_id),
    UNIQUE (tenant_id, alias),
    CONSTRAINT catalog_status_chk CHECK (status IN ('active', 'retired')),
    CONSTRAINT catalog_alias_chk CHECK (alias ~ '^[a-z][a-z0-9_-]{0,62}$')
);
CREATE SEQUENCE IF NOT EXISTS core.catalog_id_seq;

-- ---------------------------------------------------------------------------
-- Catalog rows (RLS on core.catalog is enabled at the end so these inserts
-- do not depend on a GUC).
-- ---------------------------------------------------------------------------

INSERT INTO core.catalog (tenant_id, catalog_id, public_id, alias, name, description)
SELECT t.id, nextval('core.catalog_id_seq'), gen_random_uuid(), 'general', 'Основной',
       'Default catalog (migration backfill)'
FROM core.tenant t
ON CONFLICT (tenant_id, alias) DO NOTHING;

INSERT INTO core.catalog (tenant_id, catalog_id, public_id, alias, name, description)
SELECT t.id, nextval('core.catalog_id_seq'), gen_random_uuid(), 'sys', 'Системный',
       'Platform-owned types and reference data'
FROM core.tenant t
ON CONFLICT (tenant_id, alias) DO NOTHING;

INSERT INTO core.catalog (tenant_id, catalog_id, public_id, alias, name, description)
SELECT t.id, nextval('core.catalog_id_seq'), gen_random_uuid(), 'wh', 'Склад',
       'Warehouse domain catalog'
FROM core.tenant t
WHERE EXISTS (
    SELECT 1 FROM meta.type_definition td
    WHERE td.tenant_id = t.id AND td.stable_alias LIKE 'wh\_%'
)
ON CONFLICT (tenant_id, alias) DO NOTHING;

-- ---------------------------------------------------------------------------
-- catalog_id columns (nullable → backfill → NOT NULL → FK).
-- ---------------------------------------------------------------------------

ALTER TABLE core.entity            ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE core.statement         ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE meta.type_definition   ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE meta.predicate_definition ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE meta.schema_revision   ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE meta.workflow_definition ADD COLUMN IF NOT EXISTS catalog_id bigint;
ALTER TABLE audit.event            ADD COLUMN IF NOT EXISTS catalog_id bigint;

DO $$
DECLARE
    t uuid;
BEGIN
    FOR t IN SELECT id FROM core.tenant LOOP
        PERFORM set_config('app.tenant_id', t::text, true);

        -- Everything defaults to `general`.
        UPDATE core.entity e SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE e.catalog_id IS NULL AND c.tenant_id = t AND c.alias = 'general'
          AND e.tenant_id = t;
        UPDATE meta.type_definition td SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE td.catalog_id IS NULL AND c.tenant_id = t AND c.alias = 'general'
          AND td.tenant_id = t;
        UPDATE meta.predicate_definition pd SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE pd.catalog_id IS NULL AND c.tenant_id = t AND c.alias = 'general'
          AND pd.tenant_id = t;
        UPDATE meta.schema_revision r SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE r.catalog_id IS NULL AND c.tenant_id = t AND c.alias = 'general'
          AND r.tenant_id = t;
        UPDATE meta.workflow_definition wd SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE wd.catalog_id IS NULL AND c.tenant_id = t AND c.alias = 'general'
          AND wd.tenant_id = t;

        -- Warehouse domain rows move to `wh`.
        UPDATE meta.type_definition td SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE c.tenant_id = t AND c.alias = 'wh' AND td.tenant_id = t
          AND td.stable_alias LIKE 'wh\_%';
        UPDATE meta.predicate_definition pd SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE c.tenant_id = t AND c.alias = 'wh' AND pd.tenant_id = t
          AND pd.stable_alias LIKE 'wh\_%';
        UPDATE meta.schema_revision r SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE c.tenant_id = t AND c.alias = 'wh' AND r.tenant_id = t
          AND r.content::text LIKE '%wh\_%';
        UPDATE meta.workflow_definition wd SET catalog_id = c.catalog_id
        FROM core.catalog c
        WHERE c.tenant_id = t AND c.alias = 'wh' AND wd.tenant_id = t
          AND wd.content::text LIKE '%wh\_%';

        -- Entities adopt the catalog of their own type.
        UPDATE core.entity e SET catalog_id = td.catalog_id
        FROM meta.type_definition td
        WHERE td.tenant_id = e.tenant_id AND td.type_id = e.type_id
          AND e.tenant_id = t;

        -- Statements follow their subject entity (docs/28 §2).
        UPDATE core.statement s SET catalog_id = e.catalog_id
        FROM core.entity e
        WHERE s.catalog_id IS NULL
          AND e.tenant_id = s.tenant_id AND e.entity_id = s.subject
          AND s.tenant_id = t;
    END LOOP;
END $$;

ALTER TABLE core.entity              ALTER COLUMN catalog_id SET NOT NULL;
ALTER TABLE core.statement           ALTER COLUMN catalog_id SET NOT NULL;
ALTER TABLE meta.type_definition     ALTER COLUMN catalog_id SET NOT NULL;
ALTER TABLE meta.predicate_definition ALTER COLUMN catalog_id SET NOT NULL;
ALTER TABLE meta.schema_revision     ALTER COLUMN catalog_id SET NOT NULL;
ALTER TABLE meta.workflow_definition ALTER COLUMN catalog_id SET NOT NULL;
-- audit.event.catalog_id stays NULL-able: non-catalog operations have none.

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'entity_catalog_fk') THEN
        ALTER TABLE core.entity ADD CONSTRAINT entity_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'statement_catalog_fk') THEN
        ALTER TABLE core.statement ADD CONSTRAINT statement_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'type_def_catalog_fk') THEN
        ALTER TABLE meta.type_definition ADD CONSTRAINT type_def_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'pred_def_catalog_fk') THEN
        ALTER TABLE meta.predicate_definition ADD CONSTRAINT pred_def_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'schema_rev_catalog_fk') THEN
        ALTER TABLE meta.schema_revision ADD CONSTRAINT schema_rev_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'workflow_def_catalog_fk') THEN
        ALTER TABLE meta.workflow_definition ADD CONSTRAINT workflow_def_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'audit_event_catalog_fk') THEN
        ALTER TABLE audit.event ADD CONSTRAINT audit_event_catalog_fk
            FOREIGN KEY (tenant_id, catalog_id) REFERENCES core.catalog (tenant_id, catalog_id);
    END IF;
END $$;

-- ---------------------------------------------------------------------------
-- Indexes: namespaces and published uniqueness become per-catalog.
-- ---------------------------------------------------------------------------

CREATE INDEX IF NOT EXISTS entity_catalog
    ON core.entity (tenant_id, catalog_id) WHERE retired_at IS NULL;
CREATE INDEX IF NOT EXISTS entity_catalog_type
    ON core.entity (tenant_id, catalog_id, type_id) WHERE retired_at IS NULL;
CREATE INDEX IF NOT EXISTS statement_catalog
    ON core.statement (tenant_id, catalog_id, status);
CREATE INDEX IF NOT EXISTS audit_event_catalog
    ON audit.event (tenant_id, catalog_id) WHERE catalog_id IS NOT NULL;

DROP INDEX IF EXISTS meta.type_def_alias;
CREATE INDEX type_def_alias ON meta.type_definition (tenant_id, catalog_id, stable_alias);
DROP INDEX IF EXISTS meta.pred_def_alias;
CREATE INDEX pred_def_alias ON meta.predicate_definition (tenant_id, catalog_id, stable_alias);
DROP INDEX IF EXISTS meta.schema_revision_one_published;
CREATE UNIQUE INDEX schema_revision_one_published
    ON meta.schema_revision (tenant_id, catalog_id) WHERE state = 'published';

-- ---------------------------------------------------------------------------
-- Compatibility: every existing group keeps full catalog access — the
-- catalog gate is deny-by-default, and without these rules all existing
-- principals would lose data access on upgrade (docs/28 §4). Runs in a
-- per-tenant loop because sec.* is FORCE-RLS'd.
-- ---------------------------------------------------------------------------

DO $$
DECLARE
    t uuid;
    rev bigint;
BEGIN
    FOR t IN SELECT id FROM core.tenant LOOP
        PERFORM set_config('app.tenant_id', t::text, true);
        SELECT revision_id INTO rev FROM sec.policy_revision
         WHERE tenant_id = t AND state = 'published' LIMIT 1;
        INSERT INTO sec.policy_rule
            (tenant_id, policy_revision, group_id, action, effect, resource)
        SELECT t, COALESCE(rev, 1), g.group_id, 'data.catalog.*', 'allow',
               '{"catalog":"*"}'::jsonb
        FROM sec."group" g
        WHERE g.tenant_id = t
          -- ADR-010: only data-class groups may carry data rules. A
          -- capability-class grant would leak the catalog registry
          -- (names in /me/context and /catalogs) to every member.
          AND g.class = 'data'
          AND NOT EXISTS (
            SELECT 1 FROM sec.policy_rule r
            WHERE r.tenant_id = g.tenant_id AND r.group_id = g.group_id
              AND r.action LIKE 'data.catalog.%'
          );
    END LOOP;
END $$;

-- ---------------------------------------------------------------------------
-- RLS (enabled last — inserts above run outside the policy) + grants.
-- ---------------------------------------------------------------------------

ALTER TABLE core.catalog ENABLE ROW LEVEL SECURITY;
ALTER TABLE core.catalog FORCE ROW LEVEL SECURITY;
DROP POLICY IF EXISTS tenant_isolation_catalog ON core.catalog;
CREATE POLICY tenant_isolation_catalog ON core.catalog
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE ON core.catalog TO enginer_app, enginer_worker;
GRANT USAGE ON SEQUENCE core.catalog_id_seq TO enginer_app, enginer_worker;
