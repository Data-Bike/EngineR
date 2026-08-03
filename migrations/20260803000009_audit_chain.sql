-- audit schema: tamper-evident chain + extended fields (docs/08 §11).
--
-- Extends audit.event with:
-- - sequence: monotonic chain sequence number per tenant
-- - previous_hash: hash of previous entry (chain link)
-- - entry_hash: hash of this entry (includes previous_hash)
-- - kind: audit event kind (authentication, decision, denied_request, ...)
-- - effective_actor: for impersonation (docs/08 §12)
-- - query_fingerprint: for query operations
-- - reason_ids: policy rule IDs that contributed to decision
-- - row_count, bytes: metrics for bulk/export operations
-- - client, service: client/service identification
-- - metadata: additional minimized metadata (no PII/passwords)
--
-- Also adds:
-- - audit.chain_checkpoint: periodic checkpoints for verification
-- - audit.retention_policy: retention configuration per tenant
--
-- Audit rows remain insert-only from the application role; no UPDATE/DELETE
-- is granted to enginer_app. A separate retention role handles archival
-- (docs/08 §11, §15).

-- Add tamper-evident chain columns to audit.event.
ALTER TABLE audit.event
    ADD COLUMN IF NOT EXISTS sequence         bigint,
    ADD COLUMN IF NOT EXISTS previous_hash    text,
    ADD COLUMN IF NOT EXISTS entry_hash       text,
    ADD COLUMN IF NOT EXISTS kind             text NOT NULL DEFAULT 'decision',
    ADD COLUMN IF NOT EXISTS effective_actor  uuid,
    ADD COLUMN IF NOT EXISTS query_fingerprint text,
    ADD COLUMN IF NOT EXISTS reason_ids       uuid[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS row_count        bigint,
    ADD COLUMN IF NOT EXISTS bytes            bigint,
    ADD COLUMN IF NOT EXISTS client           text,
    ADD COLUMN IF NOT EXISTS service          text,
    ADD COLUMN IF NOT EXISTS metadata         jsonb NOT NULL DEFAULT '{}'::jsonb;

-- Add constraint: kind must be a valid audit kind.
ALTER TABLE audit.event
    DROP CONSTRAINT IF EXISTS audit_event_kind_check;
ALTER TABLE audit.event
    ADD CONSTRAINT audit_event_kind_check CHECK (kind IN (
        'authentication', 'decision', 'denied_request', 'policy_change',
        'export', 'blob_read', 'bulk_action', 'break_glass', 'impersonation'
    ));

-- Index for chain verification (tenant + sequence).
CREATE INDEX IF NOT EXISTS audit_event_chain
    ON audit.event (tenant_id, sequence);

-- Index for kind-based queries.
CREATE INDEX IF NOT EXISTS audit_event_kind
    ON audit.event (tenant_id, kind, occurred_at);

-- Index for decision-based queries.
CREATE INDEX IF NOT EXISTS audit_event_decision
    ON audit.event (tenant_id, effect, occurred_at);

-- Chain checkpoint table: periodically stores the latest hash for
-- verification (docs/08 §11).
CREATE TABLE IF NOT EXISTS audit.chain_checkpoint (
    checkpoint_id    bigint GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    tenant_id        uuid NOT NULL,
    sequence         bigint NOT NULL,
    entry_hash       text NOT NULL,
    created_at       timestamptz NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS audit_chain_checkpoint_tenant
    ON audit.chain_checkpoint (tenant_id, sequence);

-- RLS on chain_checkpoint: tenant isolation.
ALTER TABLE audit.chain_checkpoint ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit.chain_checkpoint FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_audit_chain_checkpoint
    ON audit.chain_checkpoint
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT ON audit.chain_checkpoint TO enginer_app, enginer_worker;

-- Retention policy table: per-tenant retention configuration
-- (docs/08 §11, §15).
CREATE TABLE IF NOT EXISTS audit.retention_policy (
    tenant_id              uuid PRIMARY KEY,
    retention_days         integer NOT NULL DEFAULT 365,
    legal_hold            boolean NOT NULL DEFAULT false,
    archive_before_delete boolean NOT NULL DEFAULT true,
    archive_url           text,
    updated_at            timestamptz NOT NULL DEFAULT now()
);

-- RLS on retention_policy: tenant isolation.
ALTER TABLE audit.retention_policy ENABLE ROW LEVEL SECURITY;
ALTER TABLE audit.retention_policy FORCE ROW LEVEL SECURITY;
CREATE POLICY tenant_isolation_audit_retention_policy
    ON audit.retention_policy
    USING (tenant_id::text = current_setting('app.tenant_id', true));

GRANT SELECT, INSERT, UPDATE ON audit.retention_policy TO enginer_app;
GRANT SELECT ON audit.retention_policy TO enginer_worker;

-- Retention role: can DELETE old audit rows after archival.
-- (Created by the baseline migration; here we just grant.)
GRANT DELETE ON audit.event TO enginer_retention;
GRANT SELECT ON audit.chain_checkpoint TO enginer_retention;
