-- integration schema: connectors, external_id_mapping, sync_state
-- (docs/10 §13). Phase 0+1: table skeletons; full connector runtime arrives
-- in a later phase.

CREATE TABLE IF NOT EXISTS integration.connector (
    connector_id      uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id         uuid NOT NULL,
    alias             text NOT NULL,
    kind              text NOT NULL,                 -- 'http' | 'database' | 'file' | ...
    base_url          text,
    auth_secret_ref   text,
    status            text NOT NULL DEFAULT 'disabled',  -- disabled|enabled|error
    config            jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at        timestamptz NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, alias)
);

CREATE TABLE IF NOT EXISTS integration.external_id_mapping (
    tenant_id         uuid NOT NULL,
    connector_id      uuid NOT NULL REFERENCES integration.connector(connector_id),
    external_kind     text NOT NULL,
    external_id       text NOT NULL,
    internal_kind     text NOT NULL,
    internal_id       text NOT NULL,
    first_seen_at     timestamptz NOT NULL DEFAULT now(),
    last_seen_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, connector_id, external_kind, external_id)
);

CREATE TABLE IF NOT EXISTS integration.sync_state (
    tenant_id         uuid NOT NULL,
    connector_id      uuid NOT NULL REFERENCES integration.connector(connector_id),
    sync_kind         text NOT NULL,
    last_cursor       text,
    last_synced_at    timestamptz,
    last_status       text,
    last_error        text,
    PRIMARY KEY (tenant_id, connector_id, sync_kind)
);

-- RLS: tenant isolation.
DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['connector','external_id_mapping','sync_state']
  LOOP
    EXECUTE format('ALTER TABLE integration.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE integration.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY tenant_isolation_%s ON integration.%I USING (tenant_id::text = current_setting(''app.tenant_id'', true))', t, t);
  END LOOP;
END $$;

GRANT SELECT, INSERT, UPDATE ON
    integration.connector, integration.external_id_mapping, integration.sync_state
TO enginer_app, enginer_worker;
