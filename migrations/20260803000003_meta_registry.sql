-- meta schema registry (docs/10 §4): schema_revision, type/predicate
-- definitions, localized_text. The full revision body is stored as JSONB in
-- meta.schema_revision.content for versioned reads; physical type/predicate
-- tables are populated in parallel for indexing.

CREATE TABLE IF NOT EXISTS meta.schema_revision (
    tenant_id           uuid NOT NULL,
    revision_id         bigint NOT NULL,
    public_id           uuid NOT NULL DEFAULT gen_random_uuid(),
    state               text NOT NULL DEFAULT 'draft',  -- draft|validating|impact_ready|approved|publishing|published|retired|failed
    parent_revision_id  bigint,
    semantic_version    text NOT NULL DEFAULT '0.1.0',
    content_hash        bytea,
    content             jsonb NOT NULL DEFAULT '{}'::jsonb,  -- full SchemaRevision body
    created_by          uuid,
    created_at          timestamptz NOT NULL DEFAULT now(),
    published_by        uuid,
    published_at        timestamptz,
    retired_at          timestamptz,
    migration_plan_id   uuid,
    compatibility       jsonb,
    PRIMARY KEY (tenant_id, revision_id),
    UNIQUE (tenant_id, public_id)
);
-- Only one current published revision per tenant (partial unique index).
CREATE UNIQUE INDEX IF NOT EXISTS schema_revision_one_published
    ON meta.schema_revision (tenant_id) WHERE state = 'published';
CREATE INDEX IF NOT EXISTS schema_revision_state ON meta.schema_revision (tenant_id, state);

CREATE TABLE IF NOT EXISTS meta.type_definition (
    tenant_id        uuid NOT NULL,
    type_id          uuid NOT NULL,
    stable_alias     text NOT NULL,
    entity_class     smallint NOT NULL,
    created_revision bigint NOT NULL,
    retired_revision bigint,
    system_flag      boolean NOT NULL DEFAULT false,
    PRIMARY KEY (tenant_id, type_id)
);
CREATE INDEX IF NOT EXISTS type_def_alias ON meta.type_definition (tenant_id, stable_alias);

CREATE TABLE IF NOT EXISTS meta.predicate_definition (
    tenant_id        uuid NOT NULL,
    predicate_id     uuid NOT NULL,
    stable_alias     text NOT NULL,
    is_edge          boolean NOT NULL,
    scalar_kind      text,
    confidentiality  text NOT NULL DEFAULT 'internal',
    created_revision bigint NOT NULL,
    retired_revision bigint,
    PRIMARY KEY (tenant_id, predicate_id)
);
CREATE INDEX IF NOT EXISTS pred_def_alias ON meta.predicate_definition (tenant_id, stable_alias);

-- localized_text (docs/10 §4).
CREATE TABLE IF NOT EXISTS meta.localized_text (
    tenant_id       uuid NOT NULL,
    resource_kind   text NOT NULL,                 -- 'type' | 'predicate' | 'view' | ...
    resource_id     uuid NOT NULL,
    field           text NOT NULL,                 -- 'label' | 'description' | 'help' | ...
    locale          text NOT NULL,                 -- BCP 47
    text            text NOT NULL,
    schema_revision bigint,
    PRIMARY KEY (tenant_id, resource_kind, resource_id, field, locale, schema_revision)
);

-- RLS: meta is tenant-scoped too.
ALTER TABLE meta.schema_revision ENABLE ROW LEVEL SECURITY;
ALTER TABLE meta.schema_revision FORCE ROW LEVEL SECURITY;
ALTER TABLE meta.type_definition ENABLE ROW LEVEL SECURITY;
ALTER TABLE meta.type_definition FORCE ROW LEVEL SECURITY;
ALTER TABLE meta.predicate_definition ENABLE ROW LEVEL SECURITY;
ALTER TABLE meta.predicate_definition FORCE ROW LEVEL SECURITY;
ALTER TABLE meta.localized_text ENABLE ROW LEVEL SECURITY;
ALTER TABLE meta.localized_text FORCE ROW LEVEL SECURITY;

CREATE POLICY tenant_isolation_schema_revision ON meta.schema_revision
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_type_definition ON meta.type_definition
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_predicate_definition ON meta.predicate_definition
    USING (tenant_id::text = current_setting('app.tenant_id', true));
CREATE POLICY tenant_isolation_localized_text ON meta.localized_text
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE ON
    meta.schema_revision, meta.type_definition, meta.predicate_definition, meta.localized_text
TO enginer_app, enginer_worker;
