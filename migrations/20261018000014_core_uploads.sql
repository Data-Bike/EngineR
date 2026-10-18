-- Staged upload sessions + chunks (docs/15 §1): resumable uploads with
-- per-chunk storage; completed uploads become core.file_object rows.

CREATE TABLE IF NOT EXISTS core.upload_session (
    upload_id        uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id        uuid NOT NULL,
    filename         text NOT NULL,
    content_type     text NOT NULL DEFAULT 'application/octet-stream',
    expected_size    bigint NOT NULL,
    expected_hash    text NOT NULL,
    hash_algorithm   text NOT NULL,
    chunk_size       bigint NOT NULL,
    total_chunks     integer NOT NULL,
    status           text NOT NULL DEFAULT 'initiated',
    object_id        uuid,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS upload_session_tenant ON core.upload_session (tenant_id, status);

CREATE TABLE IF NOT EXISTS core.upload_chunk (
    tenant_id  uuid NOT NULL,
    upload_id  uuid NOT NULL REFERENCES core.upload_session (upload_id) ON DELETE CASCADE,
    chunk_index integer NOT NULL,
    data       bytea NOT NULL,
    PRIMARY KEY (tenant_id, upload_id, chunk_index)
);

DO $$
DECLARE t text;
BEGIN
  FOREACH t IN ARRAY ARRAY['upload_session','upload_chunk']
  LOOP
    EXECUTE format('ALTER TABLE core.%I ENABLE ROW LEVEL SECURITY', t);
    EXECUTE format('ALTER TABLE core.%I FORCE ROW LEVEL SECURITY', t);
    EXECUTE format('CREATE POLICY tenant_isolation_%s ON core.%I USING (tenant_id::text = current_setting(''app.tenant_id'', true))', t, t);
  END LOOP;
END $$;

GRANT SELECT, INSERT, UPDATE, DELETE ON core.upload_session, core.upload_chunk
TO enginer_app, enginer_worker;
