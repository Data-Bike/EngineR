-- ops schema: outbox, inbox, job, projection_watermark (docs/02 §7, docs/10 §9).
--
-- Outbox is drained by the worker with `FOR UPDATE SKIP LOCKED` (ADR-014). The
-- outbox table is append-only; the worker updates `dispatched_at` / `completed_at`.

CREATE TABLE IF NOT EXISTS ops.outbox (
    outbox_id        bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id        uuid NOT NULL,
    event_id         uuid NOT NULL DEFAULT gen_random_uuid(),
    aggregate_kind   text NOT NULL,                  -- 'entity' | 'schema' | 'policy' | ...
    aggregate_id     text NOT NULL,
    revision         bigint NOT NULL,
    event_type       text NOT NULL,
    payload          jsonb NOT NULL,
    headers          jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at       timestamptz NOT NULL DEFAULT now(),
    available_at     timestamptz NOT NULL DEFAULT now(),
    dispatched_at    timestamptz,
    completed_at     timestamptz,
    attempts         int NOT NULL DEFAULT 0,
    last_error       text,
    consumer_id      text
);
CREATE INDEX IF NOT EXISTS outbox_pending ON ops.outbox (available_at, outbox_id)
    WHERE dispatched_at IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS outbox_event_unique ON ops.outbox (tenant_id, aggregate_kind, aggregate_id, event_id);

CREATE TABLE IF NOT EXISTS ops.inbox (
    inbox_id         bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id        uuid NOT NULL,
    source_system    text NOT NULL,
    source_event_id  text NOT NULL,
    event_type       text NOT NULL,
    payload          jsonb NOT NULL,
    received_at      timestamptz NOT NULL DEFAULT now(),
    processed_at     timestamptz,
    status           text NOT NULL DEFAULT 'pending',  -- pending|processed|failed|duplicate
    error            text,
    UNIQUE (source_system, source_event_id)
);

CREATE TABLE IF NOT EXISTS ops.job (
    job_id           uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    job_kind         text NOT NULL,
    payload          jsonb NOT NULL,
    status           text NOT NULL DEFAULT 'pending',  -- pending|running|completed|failed|cancelled
    priority         int NOT NULL DEFAULT 0,
    scheduled_at     timestamptz NOT NULL DEFAULT now(),
    started_at       timestamptz,
    completed_at     timestamptz,
    attempts         int NOT NULL DEFAULT 0,
    last_error       text,
    idempotency_key  text,
    UNIQUE (tenant_id, job_kind, idempotency_key)
);
CREATE INDEX IF NOT EXISTS job_pending ON ops.job (scheduled_at, priority) WHERE status = 'pending';

CREATE TABLE IF NOT EXISTS ops.projection_watermark (
    tenant_id        uuid NOT NULL,
    projection_name  text NOT NULL,
    last_event_id    uuid,
    last_revision    bigint,
    updated_at       timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, projection_name)
);

-- RLS: tenant isolation on outbox/inbox/job/watermark.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['outbox','inbox','job','projection_watermark']
  LOOP
    EXECUTE format('ALTER TABLE ops.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE ops.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY tenant_isolation_%s ON ops.%I USING (tenant_id::text = current_setting(''app.tenant_id'', true))', t, t);
  END LOOP;
END $$;

-- Worker role gets SELECT/UPDATE on outbox/inbox/job/watermark (no INSERT on outbox
-- from worker; INSERT is from the API app within the same transaction).
GRANT SELECT, INSERT, UPDATE, DELETE ON ops.outbox, ops.inbox, ops.job, ops.projection_watermark TO enginer_app, enginer_worker;
