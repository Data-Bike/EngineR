-- audit schema: append-only audit log (docs/08 §11, docs/10 §12).
--
-- Audit rows are insert-only from the application role; no UPDATE/DELETE is
-- granted to enginer_app. A separate retention role handles archival.

CREATE TABLE IF NOT EXISTS audit.event (
    audit_id          bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id         uuid NOT NULL,
    request_id        uuid,
    correlation_id    uuid,
    principal_id      uuid,
    session_id        uuid,
    auth_strength     text,
    action            text NOT NULL,                 -- capability string
    resource_kind     text NOT NULL,
    resource_id       uuid,
    decision_id       uuid,
    effect            text,                          -- allow|deny|...
    outcome           text NOT NULL,                 -- success|failure|denied|error
    duration_us       bigint,
    parameters        jsonb NOT NULL DEFAULT '{}'::jsonb,
    error_code        text,
    error_message     text,
    policy_revision   bigint,
    schema_revision   bigint,
    occurred_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS audit_event_tenant_time ON audit.event (tenant_id, occurred_at);
CREATE INDEX IF NOT EXISTS audit_event_principal ON audit.event (tenant_id, principal_id, occurred_at);
CREATE INDEX IF NOT EXISTS audit_event_action ON audit.event (tenant_id, action, occurred_at);
CREATE INDEX IF NOT EXISTS audit_event_resource ON audit.event (tenant_id, resource_kind, resource_id);

-- RLS: tenant isolation.
ALTER TABLE audit.event ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit.event FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_audit_event ON audit.event
    USING (tenant_id::text = current_setting('app.tenant_id', true));

-- Insert-only for app role; no UPDATE/DELETE (docs/08 §11).
GRANT INSERT, SELECT ON audit.event TO enginer_app, enginer_worker;
