-- SKR operational tables: checks, risk factors/assessments, tasks,
-- workflow instances, requests, files/documents, reports, monitors.
-- All tenant-scoped with FORCE RLS like the rest of core.*.

CREATE TABLE IF NOT EXISTS core.check_instance (
    check_id         uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    request_id       uuid,
    entity_id        bigint,
    kind             text NOT NULL,
    status           text NOT NULL DEFAULT 'pending',
    started_at       timestamptz NOT NULL DEFAULT now(),
    completed_at     timestamptz,
    actor            uuid,
    evidence         jsonb NOT NULL DEFAULT '[]'::jsonb,
    risk_level       text,
    step_up_required boolean NOT NULL DEFAULT false,
    data_denied      boolean NOT NULL DEFAULT false
);
CREATE INDEX IF NOT EXISTS check_instance_tenant ON core.check_instance (tenant_id, entity_id, status);

CREATE TABLE IF NOT EXISTS core.risk_factor (
    factor_id     uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL,
    entity_id     bigint NOT NULL,
    criterion_id  text NOT NULL,
    label         text NOT NULL,
    level         text NOT NULL,
    resolved      boolean NOT NULL DEFAULT false,
    detected_at   timestamptz NOT NULL DEFAULT now(),
    resolved_at   timestamptz,
    source_checks uuid[] NOT NULL DEFAULT '{}',
    explanation   text NOT NULL DEFAULT ''
);
CREATE INDEX IF NOT EXISTS risk_factor_tenant ON core.risk_factor (tenant_id, entity_id, resolved);

CREATE TABLE IF NOT EXISTS core.risk_assessment (
    assessment_id    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    entity_id        bigint NOT NULL,
    score            numeric(6,2) NOT NULL DEFAULT 0,
    level            text NOT NULL,
    decision         text NOT NULL DEFAULT 'pending',
    status           text NOT NULL DEFAULT 'draft',
    assessor         uuid,
    assessed_at      timestamptz,
    rationale        text,
    approval_pending boolean NOT NULL DEFAULT false,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS risk_assessment_tenant ON core.risk_assessment (tenant_id, entity_id, status);

CREATE TABLE IF NOT EXISTS core.task (
    task_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    uuid NOT NULL,
    entity_id    bigint,
    workflow_id  uuid,
    kind         text NOT NULL,
    status       text NOT NULL DEFAULT 'pending',
    priority     integer NOT NULL DEFAULT 5,
    assignee     text,
    due_date     timestamptz,
    title        text NOT NULL,
    description  text,
    sub_tasks    jsonb NOT NULL DEFAULT '[]'::jsonb,
    blocking     boolean NOT NULL DEFAULT false,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS task_tenant ON core.task (tenant_id, entity_id, status);

CREATE TABLE IF NOT EXISTS core.workflow_instance (
    workflow_id   uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id     uuid NOT NULL,
    definition_id text NOT NULL,
    entity_id     bigint,
    current_state text NOT NULL,
    terminated    boolean NOT NULL DEFAULT false,
    variables     jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS workflow_tenant ON core.workflow_instance (tenant_id, entity_id);

CREATE TABLE IF NOT EXISTS core.request (
    request_id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         uuid NOT NULL,
    entity_id         bigint,
    initiator         uuid,
    status            text NOT NULL DEFAULT 'new',
    requested_product text NOT NULL DEFAULT '',
    requested_limit   numeric(18,2),
    urgency           text NOT NULL DEFAULT 'normal',
    due_date          date,
    created_at        timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS request_tenant ON core.request (tenant_id, status);

CREATE TABLE IF NOT EXISTS core.file_object (
    file_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    uuid NOT NULL,
    name         text NOT NULL,
    content_type text NOT NULL DEFAULT 'application/octet-stream',
    size         bigint NOT NULL DEFAULT 0,
    sha256       text,
    content      bytea,
    status       text NOT NULL DEFAULT 'ready',
    created_by   uuid,
    created_at   timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS file_object_tenant ON core.file_object (tenant_id, created_at);

CREATE TABLE IF NOT EXISTS core.entity_document (
    doc_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id   uuid NOT NULL,
    entity_id   bigint NOT NULL,
    file_id     uuid REFERENCES core.file_object (file_id),
    kind        text NOT NULL DEFAULT 'other',
    title       text NOT NULL DEFAULT '',
    qualifiers  jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at  timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS entity_document_tenant ON core.entity_document (tenant_id, entity_id);

CREATE TABLE IF NOT EXISTS core.report_instance (
    report_id    uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id    uuid NOT NULL,
    kind         text NOT NULL,
    format       text NOT NULL DEFAULT 'csv',
    status       text NOT NULL DEFAULT 'pending',
    params       jsonb NOT NULL DEFAULT '{}'::jsonb,
    file_id      uuid REFERENCES core.file_object (file_id),
    created_by   uuid,
    created_at   timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);
CREATE INDEX IF NOT EXISTS report_instance_tenant ON core.report_instance (tenant_id, status);

CREATE TABLE IF NOT EXISTS integration.monitor (
    monitor_id       uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    name             text NOT NULL,
    target_entity_id bigint,
    frequency        text NOT NULL DEFAULT 'daily',
    status           text NOT NULL DEFAULT 'active',
    next_run_at      timestamptz,
    last_run_at      timestamptz,
    last_result      jsonb,
    config           jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS monitor_tenant ON integration.monitor (tenant_id, status);

-- RLS on all new tables.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['check_instance','risk_factor','risk_assessment','task',
                           'workflow_instance','request','file_object',
                           'entity_document','report_instance']
  LOOP
    EXECUTE format('ALTER TABLE core.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE core.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY tenant_isolation_%s ON core.%I USING (tenant_id::text = current_setting(''app.tenant_id'', true))', t, t);
  END LOOP;
END $$;

ALTER TABLE integration.monitor ENABLE ROW LEVEL SECURITY;
ALTER TABLE integration.monitor FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_monitor ON integration.monitor
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE, DELETE ON
    core.check_instance, core.risk_factor, core.risk_assessment, core.task,
    core.workflow_instance, core.request, core.file_object,
    core.entity_document, core.report_instance
TO enginer_app, enginer_worker;

GRANT SELECT, INSERT, UPDATE, DELETE ON integration.monitor
TO enginer_app, enginer_worker;

-- DELETE on integration.connector for connector CRUD.
GRANT DELETE ON integration.connector TO enginer_app;
